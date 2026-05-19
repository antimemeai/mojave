TEX = pdflatex -interaction=nonstopmode -halt-on-error

cross-eval.pdf: cross-eval.tex cross-eval-config.tex evals.csv
	$(TEX) cross-eval.tex
	$(TEX) cross-eval.tex

.PHONY: clean veryclean
clean:
	rm -f *.aux *.log *.out *.toc
veryclean: clean
	rm -f cross-eval.pdf
