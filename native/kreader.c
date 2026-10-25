/* kreader: line-protocol document engine over MuPDF. Reads commands on stdin, answers on stdout.
 *   open <w> <h> <em> <path...>      lay out for a w x h point page, em pt font  -> ok <pages>
 *   page <n> <out.pgm> <scale> <dx> <dy> <pw> <ph>
 *                                    render page n (0-based) to a pw x ph gray PGM -> ok
 *   toc                              -> lines "<level>\t<page>\t<title>", then "."
 *   words <n>                        -> lines "<x0> <y0> <x1> <y1>\t<word>" (page points), then "."
 *   search <text>                    -> lines "<page>", then "."
 *   title                            -> ok <title>
 *   style <family 0|1|2> <line-height*100> <justify 0|1>   0 publisher, 1 Plex Serif, 2 Plex Sans (before open)
 *   quit
 */
#include <mupdf/fitz.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static fz_context *ctx;
static const char *FONTDIR = "/var/local/kos/fonts";

/* "KOS Serif" / "KOS Sans" resolve to IBM Plex files shipped next to the app. */
static fz_font *load_font(fz_context *c, const char *name, int bold, int italic, int needs_exact)
{
	const char *fam = strstr(name, "KOS Sans") ? "IBMPlexSans" : strstr(name, "KOS Serif") ? "IBMPlexSerif" : NULL;
	char path[1200];
	fprintf(stderr, "load_font(%s,%d,%d,%d) -> %s\n", name, bold, italic, needs_exact, fam ? fam : "none");
	if (!fam) return NULL;
	snprintf(path, sizeof path, "%s/%s-%s.ttf", FONTDIR, fam, bold ? (italic ? "BoldItalic" : "Bold") : (italic ? "Italic" : "Regular"));
	return fz_new_font_from_file(c, name, path, 0, 0);
}
static fz_document *doc;
static int npages;

static void reply_err(const char *m) { printf("err %s\n", m); fflush(stdout); }

static void do_open(char *path, float w, float h, float em)
{
	fz_try(ctx) {
		if (doc) fz_drop_document(ctx, doc);
		doc = fz_open_document(ctx, path);
		if (fz_is_document_reflowable(ctx, doc))
			fz_layout_document(ctx, doc, w, h, em);
		npages = fz_count_pages(ctx, doc);
	} fz_catch(ctx) { doc = NULL; reply_err(fz_caught_message(ctx)); return; }
	printf("ok %d\n", npages); fflush(stdout);
}

static void do_page(int n, const char *out, float scale, float dx, float dy, int pw, int ph)
{
	fz_page *page = NULL; fz_pixmap *pix = NULL; fz_device *dev = NULL;
	fz_try(ctx) {
		page = fz_load_page(ctx, doc, n);
		fz_rect b = fz_bound_page(ctx, page);
		float s = scale;
		if (!fz_is_document_reflowable(ctx, doc)) { /* fixed layout: fit to the box */
			float sx = (pw - 2 * dx) / (b.x1 - b.x0), sy = (ph - 2 * dy) / (b.y1 - b.y0);
			s = sx < sy ? sx : sy;
			dx = (pw - (b.x1 - b.x0) * s) / 2; dy = (ph - (b.y1 - b.y0) * s) / 2;
		}
		fz_matrix m = fz_concat(fz_scale(s, s), fz_translate(dx, dy));
		pix = fz_new_pixmap(ctx, fz_device_gray(ctx), pw, ph, NULL, 0);
		fz_clear_pixmap_with_value(ctx, pix, 255);
		dev = fz_new_draw_device(ctx, fz_identity, pix);
		fz_run_page(ctx, page, dev, m, NULL);
		fz_close_device(ctx, dev);
		fz_save_pixmap_as_pnm(ctx, pix, out);
	} fz_always(ctx) {
		fz_drop_device(ctx, dev); fz_drop_pixmap(ctx, pix); fz_drop_page(ctx, page);
	} fz_catch(ctx) { reply_err(fz_caught_message(ctx)); return; }
	printf("ok\n"); fflush(stdout);
}

static void print_outline(fz_outline *o, int level)
{
	for (; o; o = o->next) {
		int pg = o->page.page;
		if (pg < 0 && o->uri) {
			fz_location loc = fz_resolve_link(ctx, doc, o->uri, NULL, NULL);
			pg = fz_page_number_from_location(ctx, doc, loc);
		}
		printf("%d\t%d\t%s\n", level, pg, o->title ? o->title : "");
		print_outline(o->down, level + 1);
	}
}

static void do_toc(void)
{
	fz_outline *o = NULL;
	fz_try(ctx) { o = fz_load_outline(ctx, doc); print_outline(o, 0); }
	fz_catch(ctx) { }
	fz_drop_outline(ctx, o);
	printf(".\n"); fflush(stdout);
}

static void do_words(int n)
{
	fz_page *page = NULL; fz_stext_page *st = NULL;
	fz_try(ctx) {
		page = fz_load_page(ctx, doc, n);
		st = fz_new_stext_page_from_page(ctx, page, NULL);
		for (fz_stext_block *b = st->first_block; b; b = b->next) {
			if (b->type != FZ_STEXT_BLOCK_TEXT) continue;
			for (fz_stext_line *l = b->u.t.first_line; l; l = l->next) {
				char word[256]; int wl = 0; fz_rect r = fz_empty_rect;
				for (fz_stext_char *c = l->first_char; ; c = c ? c->next : NULL) {
					int end = !c || c->c == ' ';
					if (end) {
						if (wl) { word[wl] = 0; printf("%g %g %g %g\t%s\n", r.x0, r.y0, r.x1, r.y1, word); }
						wl = 0; r = fz_empty_rect;
						if (!c) break;
						continue;
					}
					fz_rect cb = fz_rect_from_quad(c->quad);
					r = wl ? fz_union_rect(r, cb) : cb;
					char u[8]; int k = fz_runetochar(u, c->c);
					if (wl + k < 255) { memcpy(word + wl, u, k); wl += k; }
				}
			}
		}
	} fz_always(ctx) { fz_drop_stext_page(ctx, st); fz_drop_page(ctx, page); }
	fz_catch(ctx) { }
	printf(".\n"); fflush(stdout);
}

static void do_search(const char *needle)
{
	fz_quad hits[1];
	for (int i = 0; i < npages; i++) {
		int c = 0;
		fz_try(ctx) { int mark = 0; c = fz_search_page_number(ctx, doc, i, needle, &mark, hits, 1); }
		fz_catch(ctx) { c = 0; }
		if (c) printf("%d\n", i);
	}
	printf(".\n"); fflush(stdout);
}

static void do_title(void)
{
	char buf[512] = "";
	fz_try(ctx) { fz_lookup_metadata(ctx, doc, FZ_META_INFO_TITLE, buf, sizeof buf); }
	fz_catch(ctx) { }
	printf("ok %s\n", buf); fflush(stdout);
}

int main(void)
{
	char line[4096];
	ctx = fz_new_context(NULL, NULL, 64 << 20);
	if (!ctx) return 1;
	fz_try(ctx) {
		fz_register_document_handlers(ctx);
		fz_install_load_system_font_funcs(ctx, load_font, NULL, NULL);
	}
	fz_catch(ctx) return 1;
	while (fgets(line, sizeof line, stdin)) {
		line[strcspn(line, "\r\n")] = 0;
		char path[2048], out[2048], word[1024];
		float a, b, c, d, e; int n, pw, ph;
		int off = 0;
		if (sscanf(line, "open %f %f %f %n", &a, &b, &c, &off) >= 3 && off) {
			snprintf(path, sizeof path, "%s", line + off);
			do_open(path, a, b, c);
		} else if (sscanf(line, "style %d %d %d", &n, &pw, &ph) == 3) {
			char css[1024];
			/* generic families resolve to MuPDF's built-in fonts, which this build replaces with IBM Plex */
			const char *fam = n == 1 ? "font-family:serif !important;" : n == 2 ? "font-family:sans-serif !important;" : "";
			snprintf(css, sizeof css, "body,p,div,span,li,td,blockquote,em,i,b,strong,a,h1,h2,h3,h4,h5,h6{%s} "
				"p,li,blockquote{line-height:%d.%02d !important;%s}", fam, pw / 100, pw % 100, ph ? "text-align:justify !important;" : "");
			if (n == 0 && pw == 0 && !ph) css[0] = 0;
			fz_set_user_css(ctx, css);
			printf("ok\n"); fflush(stdout);
		} else if (!doc && strncmp(line, "quit", 4)) {
			reply_err("no document");
		} else if (sscanf(line, "page %d %2047s %f %f %f %d %d", &n, out, &a, &b, &c, &pw, &ph) == 7) {
			do_page(n, out, a, b, c, pw, ph);
		} else if (!strcmp(line, "toc")) {
			do_toc();
		} else if (sscanf(line, "words %d", &n) == 1) {
			do_words(n);
		} else if (!strncmp(line, "search ", 7)) {
			snprintf(word, sizeof word, "%s", line + 7); do_search(word);
		} else if (!strcmp(line, "title")) {
			do_title();
		} else if (!strcmp(line, "quit")) {
			break;
		} else {
			reply_err("bad command");
		}
		(void)d; (void)e;
	}
	return 0;
}
