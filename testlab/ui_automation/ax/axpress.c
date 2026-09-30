#include <ApplicationServices/ApplicationServices.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
static void cfstr(CFTypeRef v, char* buf, size_t n) {
    buf[0]=0;
    if (v && CFGetTypeID(v) == CFStringGetTypeID()) CFStringGetCString(v, buf, n, kCFStringEncodingUTF8);
}
static AXUIElementRef found = NULL;
static const char* want = NULL;
static int press_target(AXUIElementRef el, int depth) {
    if (found || depth > 10) return 0;
    CFTypeRef title=NULL, desc=NULL;
    AXUIElementCopyAttributeValue(el, kAXTitleAttribute, &title);
    AXUIElementCopyAttributeValue(el, kAXDescriptionAttribute, &desc);
    char tb[128]={0};
    cfstr(title, tb, 128);
    if (strlen(tb) && !strcmp(tb, want)) { found = (AXUIElementRef)CFRetain(el); return 1; }
    CFTypeRef kids=NULL;
    if (AXUIElementCopyAttributeValue(el, kAXChildrenAttribute, &kids)==kAXErrorSuccess && kids) {
        CFIndex n = CFArrayGetCount(kids);
        for (CFIndex i=0;i<n && i<60 && !found;i++) press_target((AXUIElementRef)CFArrayGetValueAtIndex(kids,i), depth+1);
        CFRelease(kids);
    }
    return 0;
}
int main(int argc, char** argv) {
    setbuf(stdout, NULL);
    pid_t pid = (pid_t)atoi(argv[1]);
    want = argv[2];
    AXUIElementRef app = AXUIElementCreateApplication(pid);
    CFTypeRef win = NULL;
    AXError e;
    int got = 0;
    for (int t=0; t<5 && !got; t++) {
        e = AXUIElementCopyAttributeValue(app, kAXFocusedWindowAttribute, &win);
        if (e == kAXErrorSuccess && win) got = 1; else { usleep(300000); }
    }
    if (!got) { printf("no window (err=%d)\n", e); return 1; }
    printf("window ok\n");
    press_target((AXUIElementRef)win, 0);
    if (!found) { printf("NOT FOUND: %s\n", want); return 2; }
    e = AXUIElementPerformAction(found, kAXPressAction);
    printf("AXPress(%s) err=%d\n", want, e);
    return (e == kAXErrorSuccess) ? 0 : 3;
}
