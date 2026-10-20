import zlib
import struct
import math
import os

def write_png(width, height, rgba_data, filepath):
    raw_data = bytearray()
    for y in range(height):
        raw_data.append(0)  # Filter byte: None
        start = y * width * 4
        raw_data.extend(rgba_data[start:start + width * 4])
    
    compressed = zlib.compress(bytes(raw_data), level=9)
    
    png = bytearray(b'\x89PNG\r\n\x1a\n')
    ihdr_data = struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)
    ihdr_crc = zlib.crc32(b'IHDR' + ihdr_data) & 0xffffffff
    png.extend(struct.pack('>I', 13) + b'IHDR' + ihdr_data + struct.pack('>I', ihdr_crc))
    idat_crc = zlib.crc32(b'IDAT' + compressed) & 0xffffffff
    png.extend(struct.pack('>I', len(compressed)) + b'IDAT' + compressed + struct.pack('>I', idat_crc))
    iend_crc = zlib.crc32(b'IEND') & 0xffffffff
    png.extend(struct.pack('>I', 0) + b'IEND' + struct.pack('>I', iend_crc))
    
    os.makedirs(os.path.dirname(os.path.abspath(filepath)), exist_ok=True)
    with open(filepath, 'wb') as f:
        f.write(png)

def point_in_polygon(px, py, poly):
    n = len(poly)
    inside = False
    p1x, p1y = poly[0]
    for i in range(n + 1):
        p2x, p2y = poly[i % n]
        if py > min(p1y, p2y):
            if py <= max(p1y, p2y):
                if px <= max(p1x, p2x):
                    if p1y != p2y:
                        xinters = (py - p1y) * (p2x - p1x) / (p2y - p1y) + p1x
                    if p1x == p2x or px <= xinters:
                        inside = not inside
        p1x, p1y = p2x, p2y
    return inside

def generate_minimalist_v_icon(size):
    rgba = bytearray(size * size * 4)
    cx, cy = size / 2.0, size / 2.0
    r_squircle = size * 0.44

    # Polygon points of the clean "V" normalized to [0, 1]
    v_poly = [
        (0.27, 0.29),
        (0.50, 0.73),
        (0.73, 0.29),
        (0.61, 0.29),
        (0.50, 0.52),
        (0.39, 0.29),
    ]
    # Scale to pixels
    poly_px = [(p[0] * size, p[1] * size) for p in v_poly]

    for y in range(size):
        for x in range(size):
            dx = (x + 0.5 - cx) / r_squircle
            dy = (y + 0.5 - cy) / r_squircle
            
            # Superellipse squircle distance
            dist_sq = (abs(dx) ** 3.5 + abs(dy) ** 3.5) ** (1.0 / 3.5)
            
            # Antialiasing for squircle edge
            delta = (1.0 / r_squircle) * 1.5
            if dist_sq > 1.0 + delta:
                alpha = 0.0
            elif dist_sq < 1.0 - delta:
                alpha = 1.0
            else:
                alpha = (1.0 + delta - dist_sq) / (2.0 * delta)
            
            if alpha <= 0.0:
                continue

            # Dark, elegant charcoal/obsidian background
            r_bg, g_bg, b_bg = 18, 21, 30

            # 4x multisampling for the V shape antialiasing
            samples_in = 0
            for sy in (-0.25, 0.25):
                for sx in (-0.25, 0.25):
                    if point_in_polygon(x + 0.5 + sx, y + 0.5 + sy, poly_px):
                        samples_in += 1

            v_coverage = samples_in / 4.0

            r = int(r_bg * (1 - v_coverage) + 255 * v_coverage)
            g = int(g_bg * (1 - v_coverage) + 255 * v_coverage)
            b = int(b_bg * (1 - v_coverage) + 255 * v_coverage)

            idx = (y * size + x) * 4
            rgba[idx] = r
            rgba[idx + 1] = g
            rgba[idx + 2] = b
            rgba[idx + 3] = int(alpha * 255)

    return rgba

for s, path in [
    (32, 'src-tauri/icons/32x32.png'),
    (128, 'src-tauri/icons/128x128.png'),
    (256, 'src-tauri/icons/128x128@2x.png'),
    (512, 'src-tauri/icons/icon.png'),
    (64, 'static/favicon.png'),
    (64, 'build/favicon.png'),
]:
    data = generate_minimalist_v_icon(s)
    write_png(s, s, data, path)
    print(f'Generated: {path} ({s}x{s})')
