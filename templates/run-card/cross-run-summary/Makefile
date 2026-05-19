# Cross-Run Eval Summary -- build helper
TEX = pdflatex -interaction=nonstopmode -halt-on-error

eval-summary.pdf: eval-summary.tex eval-summary-config.tex runs.csv domain-pooled.csv
	$(TEX) eval-summary.tex
	$(TEX) eval-summary.tex     # 2nd pass: longtable widths + running header

.PHONY: clean veryclean
clean:
	rm -f *.aux *.log *.out *.toc
veryclean: clean
	rm -f eval-summary.pdf pg-*.png
