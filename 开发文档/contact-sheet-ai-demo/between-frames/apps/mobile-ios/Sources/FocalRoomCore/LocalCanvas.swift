import Foundation

// 本机画布节点：photo 引用项目照片，note 为自由文本；位置/尺寸为画布坐标。
public struct LocalCanvasNode: Codable, Sendable, Identifiable, Equatable {
    public enum Kind: String, Codable, Sendable { case photo, note }

    public let id: String
    public let kind: Kind
    public let photoID: String?
    public internal(set) var text: String?
    public internal(set) var x: Double
    public internal(set) var y: Double
    public internal(set) var width: Double
    public internal(set) var height: Double
    public internal(set) var zOrder: Int

    public init(id: String, kind: Kind, photoID: String? = nil, text: String? = nil,
                x: Double, y: Double, width: Double, height: Double, zOrder: Int) {
        self.id = id
        self.kind = kind
        self.photoID = photoID
        self.text = text
        self.x = x
        self.y = y
        self.width = width
        self.height = height
        self.zOrder = zOrder
    }
}

// 批注：锚定节点或整块画布的短文本。
public struct LocalCanvasAnnotation: Codable, Sendable, Identifiable, Equatable {
    public let id: String
    public let nodeID: String?
    public internal(set) var body: String

    public init(id: String, nodeID: String?, body: String) {
        self.id = id
        self.nodeID = nodeID
        self.body = body
    }
}

public struct LocalCanvas: Codable, Sendable, Equatable {
    public internal(set) var nodes: [LocalCanvasNode]
    public internal(set) var annotations: [LocalCanvasAnnotation]

    public init(nodes: [LocalCanvasNode] = [], annotations: [LocalCanvasAnnotation] = []) {
        self.nodes = nodes
        self.annotations = annotations
    }

    public static let nodeLimit = 2_000
    public static let annotationLimit = 2_000
    static let coordinateLimit = 100_000.0
    static let minSide = 40.0
    static let maxSide = 4_000.0
    public static let textLimit = 2_000

    func validated(photoIDs: Set<String>) throws {
        let nodeIDs = Set(nodes.map(\.id))
        guard nodes.count <= Self.nodeLimit, annotations.count <= Self.annotationLimit,
              nodeIDs.count == nodes.count,
              Set(annotations.map(\.id)).count == annotations.count
        else { throw LocalLibraryError.corruptLibrary }
        for node in nodes {
            let noteTextSane = node.kind != .note || ((node.text ?? "").count <= Self.textLimit)
            guard UUID(uuidString: node.id) != nil,
                  node.kind == .photo || node.kind == .note,
                  abs(node.x) <= Self.coordinateLimit, abs(node.y) <= Self.coordinateLimit,
                  node.width >= Self.minSide, node.width <= Self.maxSide,
                  node.height >= Self.minSide, node.height <= Self.maxSide,
                  node.zOrder >= 0, node.zOrder <= 1_000_000,
                  node.kind == .note || (node.photoID != nil && photoIDs.contains(node.photoID!)),
                  node.kind == .photo || node.text != nil,
                  noteTextSane,
                  node.kind == .photo || node.photoID == nil
            else { throw LocalLibraryError.corruptLibrary }
        }
        for annotation in annotations {
            guard UUID(uuidString: annotation.id) != nil,
                  annotation.body.count <= Self.textLimit,
                  annotation.nodeID == nil || nodeIDs.contains(annotation.nodeID!)
            else { throw LocalLibraryError.corruptLibrary }
        }
    }
}

// 画布操作：applyCanvasOperation 应用后返回其逆操作，调用方以逆操作驱动撤销/重做。
public enum LocalCanvasOperation: Sendable, Equatable {
    case createNode(LocalCanvasNode)
    case removeNode(id: String)
    case restoreNode(LocalCanvasNode, annotations: [LocalCanvasAnnotation])
    case moveNode(id: String, x: Double, y: Double)
    case resizeNode(id: String, width: Double, height: Double)
    case updateNoteText(id: String, text: String)
    case bringToFront(id: String)
    case setNodeOrder(id: String, zOrder: Int)
    case createAnnotation(LocalCanvasAnnotation)
    case removeAnnotation(id: String)
    case updateAnnotationBody(id: String, body: String)
}
