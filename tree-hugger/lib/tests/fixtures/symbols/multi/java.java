// Multi-file fixture: Java.

public class Counter {
    private int count;

    public Counter(int start) {
        this.count = start;
    }

    public void increment() {
        count++;
    }

    public static Counter zero() {
        return new Counter(0);
    }
}

interface Resettable {
    void reset();
}
