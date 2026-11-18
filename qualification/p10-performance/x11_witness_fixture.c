/* A native test client. The X server displays a new color after a real click. */
#include <X11/Xlib.h>
#include <stdio.h>

int main(void) {
    Display *display = XOpenDisplay(NULL);
    if (display == NULL) {
        fputs("open private display failed\n", stderr);
        return 1;
    }
    Window window = XCreateSimpleWindow(display, DefaultRootWindow(display), 200,
                                        160, 200, 150, 0, 0, 0x102040);
    XSetWindowAttributes attributes = {.override_redirect = True};
    XChangeWindowAttributes(display, window, CWOverrideRedirect, &attributes);
    XSelectInput(display, window, ButtonPressMask | ButtonReleaseMask);
    XMapRaised(display, window);
    XSync(display, False);
    if (puts("mapped") == EOF || fflush(stdout) == EOF) {
        return 1;
    }

    for (;;) {
        XEvent event;
        XNextEvent(display, &event);
        if (event.type == ButtonRelease) {
            XSetWindowBackground(display, window, 0x884422);
            XClearWindow(display, window);
            XFlush(display);
        }
    }
}
