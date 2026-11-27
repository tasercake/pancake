"""Finder layout for the drag-to-Applications disk image."""

from pathlib import Path

# dmgbuild supplies defines from its -D arguments.
files = [defines["app"]]
symlinks = {"Applications": "/Applications"}
format = "UDZO"

background = "builtin-arrow"
window_rect = ((100, 100), (640, 240))
default_view = "icon-view"
show_sidebar = False
show_toolbar = False
show_status_bar = False
show_pathbar = False
show_tab_view = False
icon_locations = {Path(defines["app"]).name: (140, 120), "Applications": (500, 120)}
icon_size = 112
text_size = 16
