CFLAGS=-g -O0 -Wall -fPIC
LDFLAGS=-ldl

nvml: nvml.o sonar-nvml.o

sonar-nvml.o: sonar-nvml.c Makefile
	$(CC) -c $(CFLAGS) -o sonar-nvml.o sonar-nvml.c

sonar-nvml.a: sonar-nvml.o
	ar r sonar-nvml.a sonar-nvml.o
	ranlib sonar-nvml.a
