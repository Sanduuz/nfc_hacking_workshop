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

AUTH_AID = "66556E"
AUTH_FID = 18
AUTH_KEY = "4b3379315f4630525f41757468316e47"
AUTH_DATA = b"1fY0uC4nR34d7hisPl3aseL3tMeKn0w"


def read_file(
    application_id: str, file_id: int, key_id: int = 0, key: str = "00" * 16
) -> str:
    proxmark.console(
        f"hf mfdes read --aid {application_id} --fid {file_id:02X} -n {key_id} -k {key}"
    )
    return proxmark.grabbed_output


def parse_file_contents(data: str) -> bytes:
    hex_bytes = "".join(
        re.findall(r"\[=\]\s*[\d]*\/\dx[\d]{2} \| (.*)\ \|", data)
    ).replace(" ", "")

    return bytes.fromhex(hex_bytes)


def valid_card_in_field() -> bool:
    """
    Check whether a valid ISO 14443-A card is in the reader field.

    A valid card in this function means it's a ISO 14443-A card
    that responds with ATQA 44 03 (Mifare DESFire).

    :returns: Whether a valid card is in reader field
    """
    proxmark.console("hf 14a raw -ab7 52")  # Raw 7-bit WUPA request
    ATQA = proxmark.grabbed_output.strip()
    return ATQA == "[+] 44 03"


def main():
    window = reader_status_indicator.init_window()
    while not window.closed():
        try:
            if valid_card_in_field():
                logging.info("Found valid card.")
                logging.info("Reading auth data...")
                contents = read_file(AUTH_AID, AUTH_FID, key_id=1, key=AUTH_KEY)
                parsed_contents = parse_file_contents(contents)
                if parsed_contents == AUTH_DATA:
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
    parser = argparse.ArgumentParser("Mifare DESFire - Reader")

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
