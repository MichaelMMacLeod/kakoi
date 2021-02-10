include config.mk

.PHONY: open_and_watch open compile reload watch

open_and_watch: open watch

open:
	$(BROWSER) build/index.html

compile:
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
	ls -d src/* | entr sh -c 'make reload'
