DLG_DIR=assets/dialogue
DLG_NAME=test
DLG_OUT=$(DLG_DIR)/build/$(DLG_NAME).yarnc

run: $(DLG_OUT)
	@cargo run --features bevy/dynamic_linking

dialogue_run:
	@ysc run $(DLG_DIR)/$(DLG_NAME).yarn

web: $(DLG_OUT)
	@cargo build --release --target wasm32-unknown-unknown
	@wasm-bindgen --no-typescript --out-name bevy_game --out-dir wasm --target web target/wasm32-unknown-unknown/release/strawbevy-jam.wasm
	@cp -r assets wasm/

clean:
	@rm -rf $(DLG_DIR)/build
clean-all: clean
	@rm -rf target

$(DLG_OUT): $(DLG_DIR)/$(DLG_NAME).yarn
	@mkdir -p $(DLG_DIR)/build
	@ysc compile $< --output-name=$(DLG_DIR)/build/$(DLG_NAME) --output-string-table-name=$(DLG_DIR)/build/$(DLG_NAME).yarnl --output-metadata-table-name=$(DLG_DIR)/build/$(DLG_NAME).yarnm

.PHONY: run dialogue_run web clean clean-all
