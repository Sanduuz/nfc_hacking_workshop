#!/usr/bin/env python3

import time

import reader_status_indicator

colors = [
    (255, 0, 0, 0),  # red
    (0, 255, 0, 0),  # green
    (0, 0, 255, 0),  # blue
]
window = reader_status_indicator.init_window()
while not window.closed():
    for color in colors:
        window.set_color(*color)
        time.sleep(1)
