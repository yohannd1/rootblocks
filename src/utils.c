#include "bindgen.h"

int utils_x_default_screen(Display *dpy) {
	return DefaultScreen(dpy);
}

Window utils_x_root_window(Display *dpy, int screen) {
	return RootWindow(dpy, screen);
}
