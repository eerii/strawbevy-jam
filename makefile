DLG_DIR=assets/dialogue
DLG_NAME=test
DLG_OUT=$(DLG_DIR)/build/$(DLG_NAME).yarnc

run: $(DLG_OUT)
	@cargo run --features bevy/dynamic_linking

dialogue_run:
	@ysc run $(DLG_DIR)/$(DLG_NAME).yarn

clean:
	@rm -rf $(DLG_DIR)/build
clean-all: clean
	@rm -rf target

$(DLG_OUT): $(DLG_DIR)/$(DLG_NAME).yarn
	@mkdir -p $(DLG_DIR)/build
	@ysc compile $< --output-name=$(DLG_DIR)/build/$(DLG_NAME) --output-string-table-name=$(DLG_DIR)/build/$(DLG_NAME).yarnl --output-metadata-table-name=$(DLG_DIR)/build/$(DLG_NAME).yarnm

.PHONY: run dialogue_run clean clean-all
