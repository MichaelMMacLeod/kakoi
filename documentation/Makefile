include config.mk

.PHONY: open_and_watch open compile reload watch setup

SVG_IMAGES=$(wildcard src/images/*.svg)
PNG_IMAGES=$(SVG_IMAGES:src/images/%.svg=build/images/%.png)

open_and_watch: open watch

setup:
	mkdir -p build/
	mkdir -p build/images
	touch build/index.html

open: setup
	$(BROWSER) build/index.html

build/images/%.png: src/images/%.svg
	inkscape $< \
		--export-filename $@ \
		--export-dpi 96

compile: setup $(PNG_IMAGES)
	pandoc \
		--from markdown \
		--to html \
		--output build/index.html \
		src/kakoi.md

reload: compile
	env \
		BROWSER_WINDOW_REGEXP=$(BROWSER_WINDOW_REGEXP) \
		BROWSER=$(BROWSER) \
		./reload-browser.sh

watch:
	ls -d src/* src/images/* | entr sh -c 'make reload'

clean:
	rm -rf build/
