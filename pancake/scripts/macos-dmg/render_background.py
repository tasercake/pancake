"""Render the Finder background at 1x and 2x. Requires Pillow."""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


HERE = Path(__file__).resolve().parent
FONTS = HERE.parents[1] / "crates/pancake-gui/assets/fonts"
WIDTH, HEIGHT = 720, 440
INK = "#202020"
SECONDARY = "#484848"


def render(scale):
    """Draw the install arrow and an illustrated map of the first-launch controls."""
    image = Image.new("RGB", (WIDTH * scale, HEIGHT * scale), "#f5f5f5")
    draw = ImageDraw.Draw(image)

    def text(x, y, value, size, *, strong=False, anchor="lm", fill=INK):
        weight = "SemiBold" if strong else "Regular"
        font = ImageFont.truetype(str(FONTS / f"SpaceGrotesk-{weight}.ttf"), size * scale)
        draw.text((x * scale, y * scale), value, font=font, fill=fill, anchor=anchor)

    def box(bounds, *, fill, outline=None):
        draw.rounded_rectangle(tuple(n * scale for n in bounds), radius=6 * scale,
                               fill=fill, outline=outline, width=scale)

    def line(points, *, width=2, fill=INK):
        draw.line([(x * scale, y * scale) for x, y in points],
                  fill=fill, width=width * scale, joint="curve")

    # Finder draws the live app, Applications, and help icons in the reserved areas.
    text(360, 35, "Drag Pancake Stacker to Applications", 26, strong=True, anchor="mm")
    line(((309, 120), (411, 120)), width=4, fill=SECONDARY)
    line(((395, 104), (411, 120), (395, 136)), width=4, fill=SECONDARY)
    text(360, 195, "Open Pancake Stacker from Applications.", 16, anchor="mm", fill=SECONDARY)

    box((24, 224, 565, 416), fill="#ffffff", outline="#b7b7b7")
    text(44, 251, "First launch", 22, strong=True)
    text(44, 279, "If macOS blocks Pancake Stacker, follow this path.", 15, fill=SECONDARY)

    steps = (
        (44, 193, "System Settings", "", False),
        (211, 371, "Privacy & Security", "Scroll to Security", False),
        (389, 545, "Open Anyway", "Confirm when prompted", True),
    )
    for x1, x2, label, detail, action in steps:
        center = (x1 + x2) // 2
        box((x1, 300, x2, 368), fill=INK if action else "#f3f3f3",
            outline=INK if action else "#a4a4a4")
        text(center, 321 if detail else 334, label, 15, strong=True,
             anchor="mm", fill="#ffffff" if action else INK)
        if detail:
            text(center, 346, detail, 12, anchor="mm", fill="#e8e8e8" if action else SECONDARY)
    for x in (202, 380):
        line(((x - 3, 328), (x + 3, 334), (x - 3, 340)), fill=SECONDARY)

    text(44, 394, "Continue only for a download from Pancake’s GitHub Releases.", 13, fill=SECONDARY)
    text(639, 251, "Settings shortcut", 14, strong=True, anchor="mm")
    return image


if __name__ == "__main__":
    render(1).save(HERE / "first-launch.png", optimize=True)
    render(2).save(HERE / "first-launch@2x.png", optimize=True)
