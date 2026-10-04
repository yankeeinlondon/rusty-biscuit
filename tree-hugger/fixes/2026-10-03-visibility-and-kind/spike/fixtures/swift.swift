// Symbol-kind, visibility, and static fixture for Swift.
import Foundation

open class OpenWidget {
    open func openMethod() {}
    public func publicMethod() {}
    package func packageMethod() {}
    internal func internalMethod() {}
    fileprivate func fileprivateMethod() {}
    private func privateMethod() {}
    func unmarkedMethodInOpenClass() {}

    public var publicProperty: Int = 0
    var unmarkedProperty: Int = 1
    private var privateProperty: Int = 2

    static var staticProperty: Int = 3
    class var classProperty: Int { 4 }

    static func staticMethod() -> OpenWidget { OpenWidget() }
    class func classMethod() {}
    func instanceMethod() {}

    public init() {}
}

public func publicFunction() {}

func unmarkedFunction(parameterValue: Int) -> Int {
    let localValue = parameterValue + 1
    return localValue
}

fileprivate func fileprivateFunction() {}

private func privateFunction() {}

public struct PublicApi {
    var publicApiField: Int = 0
    func unmarkedMethodInPublicStruct() {}

    private struct InnerPrivate {
        func unmarkedMethodInInnerPrivate() {}
    }
}

private struct PrivateHidden {
    var privateHiddenField: Int = 0
    func unmarkedMethodInPrivateStruct() {}
}

fileprivate struct FileprivateHidden {
    func unmarkedMethodInFileprivateStruct() {}
}

struct PlainStruct {
    var plainField: Int = 0
    func unmarkedMethodInPlainStruct() {}
}

private extension OpenWidget {
    func unmarkedMethodInPrivateExtension() {}
}

public extension OpenWidget {
    func unmarkedMethodInPublicExtension() {}
}

public enum PublicDirection {
    case north
    case south(Int)
}

enum PlainDirection {
    case east, west
}

public protocol PublicDrawable {
    func draw()
    var drawableName: String { get }
}

protocol PlainDrawable {
    func plainDraw()
}
