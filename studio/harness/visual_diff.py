#!/usr/bin/env python3
"""Compara uma captura da engine com uma imagem de referência.

Uso: python3 harness/visual_diff.py <captura.png> <referencia.png> [saida_diff.png]

Redimensiona as duas para a mesma resolução e calcula a diferença média absoluta por
pixel (0 = idênticas, 1 = totalmente diferentes). É uma métrica grosseira: serve para
acompanhar tendência entre iterações, não para declarar "igual ao Roblox".
Precisa de Pillow (pip install --break-system-packages pillow).
"""
import sys

try:
    from PIL import Image, ImageChops
except ImportError:
    print("Pillow não instalado: pip install --break-system-packages pillow")
    sys.exit(2)


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    shot = Image.open(sys.argv[1]).convert("RGB")
    ref = Image.open(sys.argv[2]).convert("RGB")
    size = (640, 360)
    shot, ref = shot.resize(size), ref.resize(size)
    diff = ImageChops.difference(shot, ref)
    pixels = list(diff.getdata())
    score = sum(sum(p) for p in pixels) / (len(pixels) * 3 * 255)
    print(f"diferença média: {score:.3f}")
    if len(sys.argv) > 3:
        diff.save(sys.argv[3])
        print(f"imagem de diferença salva em {sys.argv[3]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
