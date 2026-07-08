TEX = pdflatex
TEXFLAGS = -interaction=nonstopmode -halt-on-error
# intsys.cls и amsbib.sty лежат в соседнем worktree
TEXINPUTS_PATH = ../../../../Solver-paper:

paper_compact.pdf: paper_compact.tex
	TEXINPUTS=$(TEXINPUTS_PATH) $(TEX) $(TEXFLAGS) paper_compact.tex
	TEXINPUTS=$(TEXINPUTS_PATH) $(TEX) $(TEXFLAGS) paper_compact.tex

clean:
	rm -f *.aux *.log *.out *.toc *.pdf

.PHONY: clean
