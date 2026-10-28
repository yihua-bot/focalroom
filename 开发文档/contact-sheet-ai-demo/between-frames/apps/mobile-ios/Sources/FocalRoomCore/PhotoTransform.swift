import Foundation

public struct PhotoTransform: Sendable, Equatable {
    public private(set) var zoom: Double = 1
    public private(set) var x: Double = 0
    public private(set) var y: Double = 0

    public init() {}

    public func magnified(by factor: Double) -> Self {
        guard factor.isFinite, factor > 0 else { return self }
        var result = self
        result.zoom = min(5, max(1, zoom * factor))
        if result.zoom == 1 { result.x = 0; result.y = 0 }
        return result
    }

    public func translated(x: Double, y: Double) -> Self {
        guard zoom > 1, x.isFinite, y.isFinite else { return self }
        var result = self
        result.x = min(1, max(-1, self.x + x))
        result.y = min(1, max(-1, self.y + y))
        return result
    }
}

public struct ComparisonState: Sendable, Equatable {
    public enum Side: Sendable { case left, right }
    public private(set) var left = PhotoTransform()
    public private(set) var right = PhotoTransform()
    public private(set) var locked = true

    public init() {}

    public mutating func setLocked(_ value: Bool) {
        locked = value
        if value { right = left }
    }

    public mutating func update(_ side: Side, transform: PhotoTransform) {
        if locked { left = transform; right = transform }
        else if side == .left { left = transform }
        else { right = transform }
    }

    public mutating func reset() { left = PhotoTransform(); right = PhotoTransform() }
}
