/* Multi-file fixture: C. */

struct point {
    int x;
    int y;
};

typedef struct point point_t;

static int origin_count = 0;

int point_sum(struct point p) {
    return p.x + p.y;
}

void reset_origin(void) {
    origin_count = 0;
}
