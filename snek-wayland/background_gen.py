import struct

def main():
    with open('background.bin', 'wb') as fh:
        for x in range(0, 512):
            red_val = int(0x66 * float(x) / 512)
            for y in range(0, 512):
                green_val = int(0x66 * float(y) / 512)
                out = 0xFF000066
                out += red_val << 16
                out += green_val << 8
                fh.write(struct.pack('@I', out))

    with open('pause.bin', 'wb') as fh:
        for x in range(0, 512):
            for y in range(0, 512):
                fh.write(struct.pack('@I', 0x66336633))

if __name__ == '__main__':
    main()
