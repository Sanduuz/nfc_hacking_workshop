#!/usr/bin/env python3

import argparse
import logging
import secrets
import time
from dataclasses import dataclass

import reader_status_indicator
from Crypto.Cipher import DES3

logging.basicConfig(
    format="%(asctime)s - %(levelname)s | %(funcName)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
    level=logging.INFO,
)

KEY = bytes.fromhex("43757374306D5F332D4445535F4B6579")
START_PAGE = 14
END_PAGE = 20
AUTH_DATA_PAGE = 30
AUTH_DATA = bytes.fromhex("34553748557333444630724D46554C43")  # 4U7HUs3DF0rMFULC


def rol8(b: bytes) -> bytes:
    return b[1:] + b[:1]


def k16_to_2key_3des(k16: bytes) -> bytes:
    k1 = k16[:8]
    k2 = k16[8:]
    return k1 + k2 + k1


def des3_cbc_enc(key16: bytes, iv8: bytes, plaintext: bytes) -> bytes:
    key24 = k16_to_2key_3des(key16)
    cipher = DES3.new(key24, DES3.MODE_CBC, iv=iv8)
    return cipher.encrypt(plaintext)


def des3_cbc_dec(key16: bytes, iv8: bytes, ciphertext: bytes) -> bytes:
    key24 = k16_to_2key_3des(key16)
    cipher = DES3.new(key24, DES3.MODE_CBC, iv=iv8)
    return cipher.decrypt(ciphertext)


@dataclass
class UlcAuthStep2:
    rndb: bytes
    rndb_rol: bytes
    rnda: bytes
    m1: bytes
    c1: bytes
    frame2: bytes


def ulc_build_step2_frame(
    key16: bytes, enc_rndb: bytes, rnda: bytes | None = None
) -> UlcAuthStep2:
    iv0 = b"\x00" * 8

    rndb = des3_cbc_dec(key16, iv0, enc_rndb)
    rndb_rol = rol8(rndb)

    if rnda is None:
        rnda = secrets.token_bytes(8)

    m1 = rnda + rndb_rol
    c1 = des3_cbc_enc(key16, iv0, m1)

    frame2 = bytes([0xAF]) + c1
    return UlcAuthStep2(
        rndb=rndb, rndb_rol=rndb_rol, rnda=rnda, m1=m1, c1=c1, frame2=frame2
    )


def ulc_verify_final(key16: bytes, c1: bytes, enc_rnda_rol: bytes, rnda: bytes) -> bool:
    iv1 = c1[8:16]
    rnda_rol = des3_cbc_dec(key16, iv1, enc_rnda_rol)
    return rnda_rol == rol8(rnda)


def authenticate(key: bytes = KEY):
    proxmark.console(
        "hf 14a raw -skc 1A00"
    )  # Response: "[+] AF 00 11 22 33 44 55 66 77 88 [ XX YY ]
    response = proxmark.grabbed_output
    # logging.debug(response)
    if "incorrect" in response:
        return
    enc_rndb = response[6 : 6 + (2 + 1) * 8]  # From AF --> 2 hex bytes + space times 8
    enc_rndb_bytes = bytes.fromhex(enc_rndb.replace(" ", ""))

    step2 = ulc_build_step2_frame(key, enc_rndb_bytes)
    proxmark.console(f"hf 14a raw -akc {step2.frame2.hex()}")

    # proxmark.console("hf 14a raw -akc 300E")
    logging.debug(proxmark.grabbed_output)  # This must be done to remove the auth2 response from
                                            # grabbed_output buffer

    # Get response from command above and do this:
    # ok = ulc_verify_final(key16, step2.c1, previous_cmd_output, step2.rnda)
    # logging.debug("")
    # logging.debug("== Final response verification ==")
    # logging.debug(f"enc(RndA')  : {previous_cmd_output.hex()}")
    # logging.debug(f"Valid?      : {ok}")


def read_page(page: int) -> str:
    """This function assumes that authentication has already occured"""
    proxmark.console(f"hf 14a raw -akc 30{page:02X}")
    return proxmark.grabbed_output


def parse_page_contents(data: str) -> bytes:
    hex_bytes = data[4 : 4 + (2 + 1) * 16]  # Bytes between "[+]" and CRC.
    return bytes.fromhex(hex_bytes.replace(" ", ""))


def read_pages(start_page: int, quantity: int, key: bytes = KEY) -> None:
    """
    Read multiple pages from card.

    This is done utilising the `dump` command to read consecutive pages
    without needing to handle WUPA/ANTICOLL/SELECT/AUTH manually with raw
    commands. The `rdbl` command would do WUPA-AUTH operations for each page.

    :param start_page: Page to read onwards from
    :param quantity: Number of pages to read
    :param key: Key to authenticate with

    :return: None.
    """
    proxmark.console(
        f"hf mfu dump --page {start_page} --qty {quantity} --ns --key {key.hex()}"
    )
    return None


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
                authenticate()

                # Read flag but this is not used anywhere
                read_page(START_PAGE)
                read_page(START_PAGE + 4)

                recv_auth_data = read_page(AUTH_DATA_PAGE)
                if recv_auth_data:
                    page_contents = parse_page_contents(recv_auth_data)
                    # logging.debug(page_contents)
                    if page_contents == AUTH_DATA:
                        logging.info("Correct card provided")
                        window.set_color(0, 255, 0, 0)  # Set window to green
                        time.sleep(2)
                    else:
                        logging.info("Incorrect card provided")
                        window.set_color(255, 0, 0, 0)  # Set window to red
                        time.sleep(2)
                    window.set_color(255, 255, 0, 0)
            time.sleep(0.1)
        except KeyboardInterrupt:
            logging.info("KeyboardInterrupt caught. Exiting...")
            exit(0)
        except Exception as exc:
            logging.error(f"Unhandled exception caught: {exc}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser("Mifare Ultralight C - Reader")

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
