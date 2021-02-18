include config.mk

.PHONY: open_and_watch open compile reload watch setup

MD_FILES=$(wildcard src/*.md)
HTML_FILES=$(MD_FILES:src/%.md=build/%.html)

INDEX_HTML=$(realpath .)/build/kakoi.html

SVG_IMAGES_SRC=$(wildcard src/images/*.svg)
SVG_IMAGES=$(SVG_IMAGES_SRC:src/images/%=build/images/%)

CSS_FILES_SRC=$(wildcard src/*.css)
CSS_FILES=$(CSS_FILES_SRC:src/%=build/%)

open_and_watch: compile
	$(BROWSER) $(INDEX_HTML)
	+ls -d src/* src/images/* | entr -cp make reload

build:
	mkdir -p $@

build/images:
	mkdir -p $@

setup: build build/images

build/images/%.svg: src/images/%.svg | setup
	cp $< $@

build/%.css: src/%.css | setup
	cp $< $@

build/%.html: src/%.md | setup
	pandoc \
		--from markdown \
		--to html \
		--output $@ \
		--css=stylesheet.css \
		--standalone \
		$<

compile: $(SVG_IMAGES) $(HTML_FILES) $(CSS_FILES)

reload: compile
	env \
		BROWSER_WINDOW_REGEXP=$(BROWSER_WINDOW_REGEXP) \
		BROWSER=$(BROWSER) \
		./reload-browser.sh

clean:
	rm -rf build/
