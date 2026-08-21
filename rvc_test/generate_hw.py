import struct
with open('all_compressed.bin', 'wb') as f:
    for word in range(0x10000):
        f.write(struct.pack('<H', word))
        f.write(struct.pack('<H', 0x0001))
