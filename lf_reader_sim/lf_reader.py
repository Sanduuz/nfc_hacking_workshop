#!/usr/bin/env python3

import argparse
import logging
import re
import time

import reader_status_indicator

FC = 129
CN = 29126
REGEX_PATTERN = re.compile(r"FC:\s*(\d{1,3})\s+Card:\s*(\d{1,5})")

logging.basicConfig(
    format="%(asctime)s - %(levelname)s | %(funcName)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
    level=logging.INFO,
)


def validate_tag(card_data: str) -> bool:
    regex_match = REGEX_PATTERN.search(card_data)
    if regex_match:
        fc = int(regex_match.group(1))
        cn = int(regex_match.group(2))

        return fc == FC and cn == CN
    return False


def read_tag() -> str:
    """Returns the FC and CN in a tuple."""
    proxmark.console("lf pyramid reader")
    return proxmark.grabbed_output


def main():
    window = reader_status_indicator.init_window()
    while not window.closed():
        try:
            card_data = read_tag()
            if card_data:
                if validate_tag(card_data):
                    logging.info("Correct card provided")
                    window.set_color(0, 255, 0, 0)  # Green, correct card
                    time.sleep(2)
                else:
                    logging.info(f"Incorrect card provided: {card_data}")
                    window.set_color(255, 0, 0, 0)  # Red, wrong card
                    time.sleep(2)
                window.set_color(255, 255, 0, 0)  # Set window to yellow
            time.sleep(0.1)
        except KeyboardInterrupt:
            logging.info("KeyboardInterrupt caught. Exiting...")
            exit(0)
        except Exception as exc:
            logging.error(f"Unhandled exception caught: {exc}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser("Farpointe Pyramid LF - Reader")

    parser.add_argument(
        "-p",
        "--port",
        help="Proxmark serial port [Default: /dev/ttyACM0]",
        default="/dev/ttyACM0",
    )

    args = parser.parse_args()

    try:
        import pm3
    except ModuleNotFoundError:
        logging.critical("Failed to import the proxmark3 experimental lib. Exiting...")
        exit(1)

    proxmark = pm3.pm3(args.port)
    main()
