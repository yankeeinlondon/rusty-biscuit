// Multi-file fixture: Swift.

struct Meter {
    var reading: Int

    func doubled() -> Int {
        return reading * 2
    }
}

protocol Readable {
    func read() -> Int
}

func buildMeter() -> Meter {
    return Meter(reading: 0)
}
