// Test fixture for Swift import extraction
import Foundation
import UIKit

// Import specific symbol (Swift 5+)
// import struct Darwin.stat

class ImportsTest {
    func test() {
        let date = Date()
        print(date)
    }
}
