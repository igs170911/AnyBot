#include <errno.h>
#include <signal.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    char *end = 0;
    long pid;
    if (argc != 2) {
        return 2;
    }
    pid = strtol(argv[1], &end, 10);
    if (end == argv[1] || *end != '\0' || pid <= 0) {
        return 2;
    }
    if (kill((int)pid, SIGTERM) == 0 || errno == ESRCH) {
        return 0;
    }
    return 1;
}
