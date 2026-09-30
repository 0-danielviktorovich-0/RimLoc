#include <ApplicationServices/ApplicationServices.h>
#include <stdio.h>
#include <string.h>
static void cfstr(CFTypeRef v, char* buf, size_t n) {
    buf[0]=0;
    if (v && CFGetTypeID(v) == CFStringGetTypeID()) CFStringGetCString(v, buf, n, kCFStringEncodingUTF8);
    else if (v && CFGetTypeID(v) == CFBooleanGetTypeID()) snprintf(buf, n, "%s", CFBooleanGetValue(v) ? "true":"false");
}
static int dumped = 0;
static void walk(AXUIElementRef el, int depth) {
    if (depth > 10 || dumped > 400) return;
    CFTypeRef role=NULL, title=NULL, val=NULL, desc=NULL;
    AXUIElementCopyAttributeValue(el, kAXRoleAttribute, &role);
    AXUIElementCopyAttributeValue(el, kAXTitleAttribute, &title);
    AXUIElementCopyAttributeValue(el, kAXDescriptionAttribute, &desc);
    AXUIElementCopyAttributeValue(el, kAXValueAttribute, &val);
    char rb[48]={0}, tb[96]={0}, db[96]={0}, vb[96]={0};
    cfstr(role, rb, 48); cfstr(title, tb, 96); cfstr(desc, db, 96); cfstr(val, vb, 96);
    // печатаем только осмысленное
    if (strlen(tb) || strlen(db) || strlen(vb) || !strcmp(rb,"AXWebArea") || !strcmp(rb,"AXTabGroup")) {
        printf("%*s%s | t=%s | d=%s | v=%s\n", depth*2, "", rb, tb, db, vb);
        dumped++;
    }
    // действия
    CFArrayRef acts = NULL;
    if (AXUIElementCopyActionNames(el, &acts) == kAXErrorSuccess && acts) {
        CFIndex nacts = CFArrayGetCount(acts);
        for (CFIndex i=0;i<nacts;i++) {
            char ab[64]={0}; cfstr(CFArrayGetValueAtIndex(acts, i), ab, 64);
            if (!strcmp(ab, "AXPress")) { printf("%*s  [AXPress] t=%s d=%s\n", depth*2, "", tb, db); dumped++; }
        }
        CFRelease(acts);
    }
    CFTypeRef kids = NULL;
    if (AXUIElementCopyAttributeValue(el, kAXChildrenAttribute, &kids) == kAXErrorSuccess && kids) {
        CFIndex n = CFArrayGetCount(kids);
        for (CFIndex i=0;i<n && i<60;i++) walk((AXUIElementRef)CFArrayGetValueAtIndex(kids, i), depth+1);
        CFRelease(kids);
    }
}
int main(int argc, char** argv) {
    pid_t pid = (pid_t)atoi(argv[1]);
    AXUIElementRef app = AXUIElementCreateApplication(pid);
    CFTypeRef win = NULL;
    if (AXUIElementCopyAttributeValue(app, kAXFocusedWindowAttribute, &win) != kAXErrorSuccess) { printf("no window\n"); return 1; }
    walk((AXUIElementRef)win, 0);
    printf("printed=%d\n", dumped);
    return 0;
}
