#include <ApplicationServices/ApplicationServices.h>
#include <stdio.h>
static void describe(AXUIElementRef el, const char* label) {
    if (!el) { printf("%s: NULL\n", label); return; }
    CFTypeRef role = NULL, sub = NULL, title = NULL;
    AXUIElementCopyAttributeValue(el, kAXRoleAttribute, &role);
    AXUIElementCopyAttributeValue(el, kAXSubroleAttribute, &sub);
    AXUIElementCopyAttributeValue(el, kAXTitleAttribute, &title);
    char rb[64]={0}, sb[64]={0}, tb[128]={0};
    if (role) CFStringGetCString(role, rb, 64, kCFStringEncodingUTF8);
    if (sub) CFStringGetCString(sub, sb, 64, kCFStringEncodingUTF8);
    if (title) CFStringGetCString(title, tb, 128, kCFStringEncodingUTF8);
    printf("%s: role=%s subrole=%s title=%s\n", label, rb, sb, tb);
}
int main(int argc, char** argv) {
    pid_t pid = (pid_t)atoi(argv[1]);
    AXUIElementRef app = AXUIElementCreateApplication(pid);
    CFTypeRef v = NULL;
    AXError e1 = AXUIElementCopyAttributeValue(app, kAXWindowsAttribute, &v);
    if (e1 == kAXErrorSuccess && v) printf("AXWindows count=%ld\n", (long)CFArrayGetCount(v));
    else printf("AXWindows err=%d\n", e1);
    if (v) CFRelease(v); v = NULL;
    if (AXUIElementCopyAttributeValue(app, kAXFocusedWindowAttribute, &v) == kAXErrorSuccess) {
        describe(v ? (AXUIElementRef)v : NULL, "FocusedWindow");
        if (v) CFRelease(v);
    } else printf("FocusedWindow err\n");
    v = NULL;
    if (AXUIElementCopyAttributeValue(app, kAXMainWindowAttribute, &v) == kAXErrorSuccess) {
        describe(v ? (AXUIElementRef)v : NULL, "MainWindow");
        if (v) CFRelease(v);
    } else printf("MainWindow err\n");
    return 0;
}
