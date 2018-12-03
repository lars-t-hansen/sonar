#!/usr/bin/env python3

import sys
import os
import datetime
import time
import socket
import csv

from contextlib import contextmanager
from subprocess import check_output, SubprocessError, DEVNULL
from collections import defaultdict

import click


@contextmanager
def write_open(filename=None):
    '''
    Special wrapper to allow to write to stdout or a file nicely. If `filename` is '-' or None, everything will be written to stdout instead to a "real" file.

    Use like:
    >>> with write_open('myfile') as f:
    >>>     f.write(...)
    or
    >>> with write_open() as f:
    >>>     f.write(...)
    '''

    # https://stackoverflow.com/q/17602878
    if filename and filename != '-':
        handler = open(filename, 'w')
    else:
        handler = sys.stdout

    try:
        yield handler
    finally:
        if handler is not sys.stdout:
            handler.close()


def get_timestamp():
    '''
    Returns time stamp as string in ISO 8601 with time zone information.
    '''

    # https://stackoverflow.com/a/28147286
    utc_offset_sec = time.altzone if time.localtime().tm_isdst else time.timezone
    utc_offset = datetime.timedelta(seconds=-utc_offset_sec)

    return datetime.datetime.now().replace(tzinfo=datetime.timezone(offset=utc_offset)).isoformat()


def get_slurm_projects(hostname):
    '''
    Try to get the users and projects from the current `hostname`.
    If a user should run two jobs with two different projects, only the last discovered project will be assumed for the user.

    :returns: A defaultdict with the mapping from user to project. Project is '-' if the user is not found or slurm is not available.
    '''

    user_to_project = defaultdict(lambda: '-')

    # %a  Account (project)
    # %u  User
    try:
        output = check_output('squeue --noheader --nodelist={} --format=%a,%u'.format(hostname), shell=True, stderr=DEVNULL).decode('utf8')
    except SubprocessError:
        # if Slurm is not available, return the empty defaultdict that will return '-' for any key call.
        return user_to_project

    for line in output.split('\n'):
        line = line.strip()
        if not line:
            continue
        project, user = line.split(',')
        user_to_project[user] = project

    return user_to_project


def get_available_memory():
    '''
    Tries to return the memory available on the current node in bytes. Returns a negative number if the value cannot be determined.
    This is Unix-specific.
    '''

    # Another possibility would be to read /proc/meminfo
    return os.sysconf('SC_PAGE_SIZE') * os.sysconf('SC_PHYS_PAGES')


def get_ignored_users():
    '''
    Return a set with users that should be ignored.
    '''

    # FIXME this should be configurable
    ignored_users = {
        'avahi',
        'colord',
        'dbus',
        'haveged',
        'polkitd',
        'root',
        'rtkit'
    }

    return ignored_users


def extract_processes(raw_text, ignored_users):
    '''
    Extract user, cpu, memory, and command from `raw_text` that should be the (special) output of a `ps` command.
    `ignored_users` should be a list with users that shall be ignored.
    '''

    cpu_percentages = defaultdict(float)
    mem_percentages = defaultdict(float)
    for line in raw_text.split('\n'):
        # Using maxsplit to prevent commands to be split. This is unstable if the `ps` call is altered!
        words = line.split(maxsplit=4)
        if len(words) == 5:
            pid, user, cpu_percentage, mem_percentage, command = words
            if user not in ignored_users:
                cpu_percentages[(user, command)] += float(cpu_percentage)
                mem_percentages[(user, command)] += float(mem_percentage)

    return cpu_percentages, mem_percentages


def test_extract_processes():
    text = '''
     2011 bob                    10.0  20.0   slack
     2022 bob                    10.0  15.0   chromium
    12057 bob                    10.0  15.0   chromium
     2084 alice                  10.0   5.0   slack
     2087 bob                    10.0   5.0   someapp
     2090 alice                  10.0   5.0   someapp
     2093 alice                  10.0   5.0   someapp
    '''

    cpu_percentages, mem_percentages = extract_processes(text, set())

    assert cpu_percentages == {('bob', 'slack'): 10.0,
                               ('bob', 'chromium'): 20.0,
                               ('alice', 'slack'): 10.0,
                               ('bob', 'someapp'): 10.0,
                               ('alice', 'someapp'): 20.0}
    assert mem_percentages == {('bob', 'slack'): 20.0,
                               ('bob', 'chromium'): 30.0,
                               ('alice', 'slack'): 5.0,
                               ('bob', 'someapp'): 5.0,
                               ('alice', 'someapp'): 10.0}

    cpu_percentages, mem_percentages = extract_processes(text, ['bob'])

    assert cpu_percentages == {('alice', 'slack'): 10.0,
                               ('alice', 'someapp'): 20.0}
    assert mem_percentages == {('alice', 'slack'): 5.0,
                               ('alice', 'someapp'): 10.0}


def create_snapshot(cpu_cutoff, mem_cutoff, ignored_users):
    '''
    Take a snapshot of the currently running processes that use more than `cpu_cutoff` cpu and `mem_cutoff` memory, ignoring the set or list `ignored_users`. Return a list of lists being lines of columns.
    '''

    # -e      show all processes
    # -o      output formatting. user:30 is a hack to prevent cut-off user names
    output = check_output('ps -e --no-header -o pid,user:30,pcpu,pmem,comm', shell=True).decode('utf-8')
    timestamp = get_timestamp()
    hostname = socket.gethostname()
    # FIXME: The hostname fix is specific for Stallo. Should be possible to configure!
    hostname = hostname.split('.')[0]
    slurm_projects = get_slurm_projects(hostname)
    total_memory = get_available_memory()
    if total_memory < 0:
        total_memory = 1

    cpu_percentages, mem_percentages = extract_processes(output, ignored_users=ignored_users)

    snapshot = []

    for user, command in cpu_percentages:
        cpu_percentage = cpu_percentages[(user, command)]
        if cpu_percentage > cpu_cutoff:
            mem_percentage = mem_percentages[(user, command)]
            if mem_percentage > mem_cutoff:
                # Weird number is 1024*1024*100 to get MiB and %
                mem_absolute = int(total_memory * mem_percentage / 104857600)
                snapshot.append([timestamp, hostname, user, slurm_projects[user], command, '{:.1f}'.format(cpu_percentage), mem_absolute])

    return snapshot


def test_create_snapshot():
    snapshot = create_snapshot(0.0, 0.0, set())

    # With CPU and mem cutoffs set to 0, there should be some processes running...
    assert len(snapshot)

    first_line = snapshot[0]

    # The timestamp is always 32 characters long (until the year 10 000...)
    assert len(first_line[0]) == 32

    try:
        float(first_line[5])    # CPU
    except ValueError:
        raise AssertionError

    try:
        int(first_line[6])      # mem in MiB
    except ValueError:
        raise AssertionError


@click.command()
@click.option('--output-file', help='Output file. Leave empty or provide - for stdout (default: -).')
@click.option('--cpu-cutoff', default=0.5, help='CPU Memory consumption percentage cutoff (default: 0.5).')
@click.option('--mem-cutoff', default=0.0, help='Memory consumption percentage cutoff (default: 0.0).')
def take_snapshot(output_file,
                  cpu_cutoff,
                  mem_cutoff):
    '''
    Take a snapshot of the currently running processes that use more than `cpu_cutoff` cpu and `mem_cutoff` memory.
    '''

    ignored_users = get_ignored_users()

    snapshot = create_snapshot(cpu_cutoff, mem_cutoff, ignored_users)

    with write_open(output_file) as f:
        f_writer = csv.writer(f, delimiter='\t', quotechar='"', quoting=csv.QUOTE_MINIMAL)
        f_writer.writerows(snapshot)


if __name__ == '__main__':
    take_snapshot()