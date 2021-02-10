include config.mk

.PHONY: open_and_watch open compile reload watch setup

MD_FILES=$(wildcard src/*.md)
HTML_FILES=$(MD_FILES:src/%.md=build/%.html)

INDEX_HTML=build/kakoi.html

SVG_IMAGES=$(wildcard src/images/*.svg)
PNG_IMAGES=$(SVG_IMAGES:src/images/%.svg=build/images/%.png)

open_and_watch: compile open watch

setup:
	mkdir -p build/
	mkdir -p build/images

open: setup
	$(BROWSER) $(INDEX_HTML)

build/images/%.png: src/images/%.svg
	inkscape $< \
		--export-filename $@ \
		--export-dpi 96

build/%.html: src/%.md
	pandoc \
		--from markdown \
		--to html \
		--output $@ \
		$<

compile: setup $(PNG_IMAGES) $(HTML_FILES)

reload: compile
	env \
		BROWSER_WINDOW_REGEXP=$(BROWSER_WINDOW_REGEXP) \
		BROWSER=$(BROWSER) \
		./reload-browser.sh

watch:
	ls -d src/* src/images/* | entr sh -c 'make reload'

clean:
	rm -rf build/
