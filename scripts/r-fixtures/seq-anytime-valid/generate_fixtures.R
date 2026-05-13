library(gsDesign)
library(jsonlite)

# Pocock boundaries for K=2..5 at alpha=0.05 (two-sided)
pocock_fixtures <- lapply(2:5, function(k) {
  d <- gsDesign(k = k, test.type = 2, alpha = 0.025, beta = 0.1, sfu = "Pocock")
  list(
    K = k,
    alpha = 0.05,
    upper_bounds = d$upper$bound,
    lower_bounds = d$lower$bound,
    info_fractions = d$timing
  )
})
write_json(pocock_fixtures, "tck/seq-anytime-valid/fixtures/gsdesign_pocock.json",
           pretty = TRUE, auto_unbox = TRUE)

# OBF boundaries for K=2..5 at alpha=0.05 (two-sided)
obf_fixtures <- lapply(2:5, function(k) {
  d <- gsDesign(k = k, test.type = 2, alpha = 0.025, beta = 0.1, sfu = "OF")
  list(
    K = k,
    alpha = 0.05,
    upper_bounds = d$upper$bound,
    lower_bounds = d$lower$bound,
    info_fractions = d$timing
  )
})
write_json(obf_fixtures, "tck/seq-anytime-valid/fixtures/gsdesign_obf.json",
           pretty = TRUE, auto_unbox = TRUE)

cat("Fixtures generated.\n")
