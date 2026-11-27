"""Finder layout for the drag-to-Applications disk image and its help."""

from pathlib import Path

# dmgbuild supplies defines from its -D arguments.
app = Path(defines["app"])
assets = Path(defines["assets"])
help_page = assets / "First Launch Help.html"
files = [str(app), str(help_page)]
symlinks = {"Applications": "/Applications"}
hide_extensions = [help_page.name]
format = "UDZO"

background = str(assets / "first-launch.png")
window_rect = ((100, 100), (720, 440))
default_view = "icon-view"
show_sidebar = False
show_toolbar = False
show_status_bar = False
show_pathbar = False
show_tab_view = False
show_icon_preview = False
icon_locations = {app.name: (165, 120), "Applications": (555, 120), help_page.name: (639, 321)}
icon_size = 88
text_size = 14
