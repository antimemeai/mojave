# Mojave eval run-card templates -- build everything
.PHONY: all clean veryclean
all:
	$(MAKE) -C single-run-card
	$(MAKE) -C cross-run-summary

clean:
	$(MAKE) -C single-run-card clean
	$(MAKE) -C cross-run-summary clean

veryclean:
	$(MAKE) -C single-run-card veryclean
	$(MAKE) -C cross-run-summary veryclean
