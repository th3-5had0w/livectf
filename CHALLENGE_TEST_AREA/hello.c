#include <stdio.h>
#include <unistd.h>

int main() {
    setvbuf(stdin, 0, 2, 0);
    setvbuf(stdout, 0, 2, 0);
    setvbuf(stderr, 0, 2, 0);
    char buf[0x30]; 
    printf("what's your name? ");
    read(0, buf, 0x20);
    printf("hello %s\n", buf);
}