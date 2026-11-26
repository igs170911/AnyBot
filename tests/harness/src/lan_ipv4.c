#include <ifaddrs.h>
#include <netinet/in.h>
#include <stdio.h>
#include <arpa/inet.h>

int main(void) {
    struct ifaddrs *list = 0;
    struct ifaddrs *cursor;
    if (getifaddrs(&list) != 0) {
        return 1;
    }
    for (cursor = list; cursor != 0; cursor = cursor->ifa_next) {
        struct sockaddr_in *addr;
        char text[INET_ADDRSTRLEN];
        if (cursor->ifa_addr == 0 || cursor->ifa_addr->sa_family != AF_INET) {
            continue;
        }
        addr = (struct sockaddr_in *)cursor->ifa_addr;
        if (inet_ntop(AF_INET, &addr->sin_addr, text, sizeof text) == 0) {
            continue;
        }
        if (text[0] == '1' && text[1] == '2' && text[2] == '7' && text[3] == '.') {
            continue;
        }
        printf("%s\n", text);
        freeifaddrs(list);
        return 0;
    }
    freeifaddrs(list);
    return 1;
}
