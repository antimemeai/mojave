# Mojave Run Card -- build helper
TEX = pdflatex -interaction=nonstopmode -halt-on-error

runcard.pdf: runcard.tex runcard-config.tex domains.csv rawdata.csv
	$(TEX) runcard.tex
	$(TEX) runcard.tex          # 2nd pass: longtable widths + headers

.PHONY: clean veryclean
clean:
	rm -f *.aux *.log *.out *.toc
veryclean: clean
	rm -f runcard.pdf pg-*.png
