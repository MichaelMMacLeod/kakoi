include config.mk

.PHONY: open_and_watch open compile reload watch setup

MD_FILES=$(wildcard src/*.md)
HTML_FILES=$(MD_FILES:src/%.md=build/%.html)

INDEX_HTML=build/kakoi.html

SVG_IMAGES_SRC=$(wildcard src/images/*.svg)
SVG_IMAGES=$(SVG_IMAGES_SRC:src/images/%=build/images/%)

open_and_watch: compile open watch

setup:
	mkdir -p build/
	mkdir -p build/images

open: setup
	$(BROWSER) $(INDEX_HTML)

build/images/%.svg: src/images/%.svg
	cp $< $@

build/%.html: src/%.md
	pandoc \
		--from markdown \
		--to html \
		--output $@ \
		$<

compile: setup $(SVG_IMAGES) $(HTML_FILES)

reload: compile
	env \
		BROWSER_WINDOW_REGEXP=$(BROWSER_WINDOW_REGEXP) \
		BROWSER=$(BROWSER) \
		./reload-browser.sh

watch:
	ls -d src/* src/images/* | entr sh -c 'make reload'

clean:
	rm -rf build/
