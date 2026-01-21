/* A 2D point structure. */
struct Point {
    int x;
    int y;
};

/* Status codes for responses. */
enum Status {
    SUCCESS,
    ERROR,
    PENDING
};

/* A type alias for Point. */
typedef struct Point PointAlias;
