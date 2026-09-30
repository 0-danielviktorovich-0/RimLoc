#include <ApplicationServices/ApplicationServices.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
static void cfstr(CFTypeRef v, char* buf, size_t n) {
    buf[0]=0;
    if (v && CFGetTypeID(v) == CFStringGetTypeID()) CFStringGetCString(v, buf, n, kCFStringEncodingUTF8);
}
static AXUIElementRef found = NULL;   // любой подходящий (fallback)
static AXUIElementRef found_btn = NULL; // приоритет: роль AXButton
static const char* want = NULL;
static int press_target(AXUIElementRef el, int depth) {
    if (depth > 10) return 0;
    CFTypeRef title=NULL, role=NULL;
    AXUIElementCopyAttributeValue(el, kAXTitleAttribute, &title);
    AXUIElementCopyAttributeValue(el, kAXRoleAttribute, &role);
    char tb[128]={0}, rb[48]={0};
    cfstr(title, tb, 128); cfstr(role, rb, 48);
    if (strlen(tb) && !strcmp(tb, want)) {
        if (!found) found = (AXUIElementRef)CFRetain(el);
        if (!found_btn && !strcmp(rb, "AXButton")) found_btn = (AXUIElementRef)CFRetain(el);
    }
    CFTypeRef kids=NULL;
    if (AXUIElementCopyAttributeValue(el, kAXChildrenAttribute, &kids)==kAXErrorSuccess && kids) {
        CFIndex n = CFArrayGetCount(kids);
        for (CFIndex i=0;i<n && i<60;i++) press_target((AXUIElementRef)CFArrayGetValueAtIndex(kids,i), depth+1);
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
    AXUIElementRef target = found_btn ? found_btn : found;
    if (!target) { printf("NOT FOUND: %s\n", want); return 2; }
    e = AXUIElementPerformAction(target, kAXPressAction);
    printf("AXPress(%s)%s err=%d\n", want, found_btn ? " [btn]" : " [grp]", e);
    return (e == kAXErrorSuccess) ? 0 : 3;
}
