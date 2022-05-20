
MDBOOK ?= mdbook
PORT ?= 3000

.PHONY: book
book:
	$(MDBOOK) build

.PHONY: serve-site
serve-site: book
	python3 -m http.server -d ./pub "$(PORT)"

.PHONY: serve-book
serve-book:
	$(MDBOOK) serve -p "$(PORT)"

.PHONY: clean
clean:
	$(MDBOOK) clean
	mkdir pub/book
	touch pub/book/.gitkeep
