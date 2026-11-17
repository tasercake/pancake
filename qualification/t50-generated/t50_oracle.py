"""Independent scalar neighborhood oracle; no NumPy/prefix-sum renderer imports."""

import hashlib


def scalar_pixel(recipe, focus, x, y):
    """Calculate one RGB16 pixel by direct integer neighborhood summation."""
    depth = x * recipe.frames // recipe.width
    radius = 0 if focus is None else abs(depth - focus)
    length = 2 * radius + 1
    result = []
    for channel, bias in enumerate((32768, 30000, 35000)):
        value = bias + ((73 * x + 137 * y + recipe.seed + channel * 431) % 4096) - 2048
        for band, (period, amplitude) in enumerate(
            ((4, 12000), (14, 7000), (62, 3000))
        ):
            digest = hashlib.sha256(
                f"generated-t50/v1/{recipe.seed}/{channel}/{band}".encode()
            ).digest()
            phase_x, phase_y = digest[0] % period, digest[1] % period
            sx = sum(
                1 if (x + offset + phase_x) % period < period // 2 else -1
                for offset in range(-radius, radius + 1)
            )
            sy = sum(
                1 if (y + offset + phase_y) % period < period // 2 else -1
                for offset in range(-radius, radius + 1)
            )
            value += amplitude * sx * sy // (length * length)
        result.append(min(65535, max(0, value)))
    return result
