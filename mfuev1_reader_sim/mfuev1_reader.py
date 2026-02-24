#!/usr/bin/env python3

import argparse
import logging
import re
import time

import reader_status_indicator

logging.basicConfig(
    format="%(asctime)s - %(levelname)s | %(funcName)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
    level=logging.INFO,
)

PWD_AUTH = "4747455A"
PAGE = 14
CORRECT_DATA = "8A27C6BF"
DATA_REGEX = r"\| ([0-9A-F ]+) \|"


def read_page(page: int, pwd: str = PWD_AUTH) -> str:
    proxmark.console(f"hf mfu rdbl -b {page} -k {pwd}")
    return proxmark.grabbed_output


def parse_block_contents(content: str) -> str:
    """Parse block contents from ASCII output"""
    data = re.findall(DATA_REGEX, content)
    return data[0] if len(data) > 0 else ""


def valid_card_in_field() -> bool:
    """
    Check whether a valid ISO 14443-A card is in the reader field.

    A valid card in this function means it's a ISO 14443-A card
    that responds with ATQA 44 00 (Mifare Ultralight).

    Note: The ATQA endianness is reversed in proxmark (2025-12-30)

    :returns: Whether a valid card is in reader field
    """
    proxmark.console("hf 14a raw -ab7 52")  # Raw 7-bit WUPA request
    ATQA = proxmark.grabbed_output.strip()
    # print(f"Output: {ATQA}, {len(ATQA) = }, {type(ATQA) = }, {repr(ATQA) = }")
    return ATQA == "[+] 44 00"


def main():
    window = reader_status_indicator.init_window()
    while not window.closed():
        try:
            if valid_card_in_field():
                logging.info("Found valid card.")
                logging.info("Reading pages...")
                contents = read_page(PAGE)
                data = parse_block_contents(contents)
                if data.replace(" ", "") == CORRECT_DATA:
                    logging.info("Correct card provided")
                    window.set_color(0, 255, 0, 0)  # Set window to green
                    time.sleep(2)
                else:
                    logging.info("Incorrect card provided")
                    window.set_color(255, 0, 0, 0)  # Set window to red
                    time.sleep(2)
                window.set_color(255, 255, 0, 0)  # Set window to yellow
            time.sleep(0.1)
        except KeyboardInterrupt:
            logging.info("KeyboardInterrupt caught. Exiting...")
            exit(0)
        except Exception as exc:
            logging.error(f"Unhandled exception caught: {exc}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser("Mifare Ultralight EV1 - Reader")

    parser.add_argument(
        "-p",
        "--port",
        help="Proxmark serial port [Default: /dev/ttyACM0]",
        default="/dev/ttyACM0",
    )
    parser.add_argument("-k", "--key", help="Card Key. PWD_AUTH: 4 bytes")

    args = parser.parse_args()

    try:
        import pm3
    except ModuleNotFoundError:
        logging.critical("Failed to import the proxmark3 experimental lib. Exiting...")
        exit(1)

    proxmark = pm3.pm3(args.port)
    main()
