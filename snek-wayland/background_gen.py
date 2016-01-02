from PIL import Image
import struct

def paint_pause_screen(image):
    context = image.load()

    for y in range(0, image.width):
        for x in range(0, image.height):
            context[x, y] = (0x00, 0x00, 0x00, 0x80)


def paint_background_gradient(image):
    context = image.load()

    blue_val = 0x33
    for y in range(0, image.width):
        red_val = int(0x66 * float(x) / 512)
        for x in range(0, image.height):
            green_val = int(0x66 * float(y) / 512)
            context[x, y] = (red_val, green_val, blue_val, 0xFF)


def export_memory(fh, image):
    context = image.load()
    for y in range(0, image.width):
        for x in range(0, image.height):
            (r, g, b, a) = context[x, y]
            fh.write(struct.pack('@I', (a << 24) + (r << 16) + (g << 8) + b))


def main():
    background = Image.open('background.png')
    # paint_background_gradient(background)
    # background.save('background.ppm')
    with open('background.bin', 'wb') as fh:
        export_memory(fh, background)

    pause = Image.open('pause.png')
    # paint_pause_screen(pause)
    # pause.save('pause.ppm')
    with open('pause.bin', 'wb') as fh:
        export_memory(fh, pause)

if __name__ == '__main__':
    main()
