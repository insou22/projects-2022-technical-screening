# hard screening

* Ensure python venv is installed (on debian, `apt install python3.8-venv`
* Install `rustup` from https://rustup.rs/
* `python3 -m venv .env`
* `source .env/bin/activate`
* `pip install maturin`
* `maturin develop`
* `python3 test_hard.py`

