import SwiftUI
import AppKit
import ApplicationServices
import UniformTypeIdentifiers
import ServiceManagement
import Darwin

struct Mapping: Codable, Equatable {
    var label = ""
    var action = "none"
    var keyCode: UInt16 = 0
    var modifiers: UInt64 = 0
    var shortcut = ""
    var sensitivity = 1.0
    var invert = false
}
// Keyboard shortcut facts recovered from the publicly archived Monogram 5.9.1 presets.
// Only direct Mac key actions are included; no vendor code, artwork, or plugin actions.
struct ShortcutPreset: Decodable, Identifiable {
    let id: String
    let group: String
    let label: String
    let keyCode: UInt16
    let modifiers: UInt64
    let shortcut: String
    var mapping: Mapping { Mapping(label:label,action:"shortcut",keyCode:keyCode,modifiers:modifiers,shortcut:shortcut) }
    static let all: [ShortcutPreset] = try! JSONDecoder().decode([ShortcutPreset].self, from: Data(catalog.utf8))
    static var groups: [String] { Array(Set(all.map(\.group))).sorted() }
    private static let catalog = #"""
[{"group":"Common shortcuts","label":"Open","keyCode":31,"modifiers":1048576,"shortcut":"⌘O","id":"0"},{"group":"Common shortcuts","label":"Save","keyCode":1,"modifiers":1048576,"shortcut":"⌘S","id":"1"},{"group":"Common shortcuts","label":"Undo","keyCode":6,"modifiers":1048576,"shortcut":"⌘Z","id":"2"},{"group":"Common shortcuts","label":"Redo","keyCode":6,"modifiers":1179648,"shortcut":"⇧⌘Z","id":"3"},{"group":"Common shortcuts","label":"Undo/Redo · press","keyCode":6,"modifiers":1048576,"shortcut":"⌘Z","id":"4"},{"group":"Common shortcuts","label":"Cut","keyCode":7,"modifiers":1048576,"shortcut":"⌘X","id":"5"},{"group":"Common shortcuts","label":"Copy","keyCode":8,"modifiers":1048576,"shortcut":"⌘C","id":"6"},{"group":"Common shortcuts","label":"Paste","keyCode":9,"modifiers":1048576,"shortcut":"⌘V","id":"7"},{"group":"Common shortcuts","label":"Zoom In","keyCode":69,"modifiers":1048576,"shortcut":"⌘+ (keypad)","id":"8"},{"group":"Common shortcuts","label":"Zoom Out","keyCode":78,"modifiers":1048576,"shortcut":"⌘− (keypad)","id":"9"},{"group":"After Effects","label":"Adjust Any Property.Adjust Any Property (fine) · press","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"10"},{"group":"After Effects","label":"Adjust Any Property.Adjust Any Property (fine) · counterclockwise","keyCode":125,"modifiers":0,"shortcut":"↓","id":"11"},{"group":"After Effects","label":"Adjust Any Property.Adjust Any Property (fine) · clockwise","keyCode":126,"modifiers":0,"shortcut":"↑","id":"12"},{"group":"After Effects","label":"Adjust Any Property.Adjust Any Property (coarse) · press","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"13"},{"group":"After Effects","label":"Adjust Any Property.Adjust Any Property (coarse) · counterclockwise","keyCode":125,"modifiers":131072,"shortcut":"⇧↓","id":"14"},{"group":"After Effects","label":"Adjust Any Property.Adjust Any Property (coarse) · clockwise","keyCode":126,"modifiers":131072,"shortcut":"⇧↑","id":"15"},{"group":"After Effects","label":"Adjust Any Property.Select Property · press","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"16"},{"group":"After Effects","label":"Adjust Any Property.Select Property · counterclockwise","keyCode":48,"modifiers":131072,"shortcut":"⇧Tab","id":"17"},{"group":"After Effects","label":"Adjust Any Property.Select Property · clockwise","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"18"},{"group":"Audition","label":"Import File","keyCode":34,"modifiers":1048576,"shortcut":"⌘I","id":"19"},{"group":"Audition","label":"New Multitrack Session","keyCode":45,"modifiers":1048576,"shortcut":"⌘N","id":"20"},{"group":"Audition","label":"Add Marker","keyCode":46,"modifiers":0,"shortcut":"M","id":"21"},{"group":"Capture One","label":"Next Photo","keyCode":124,"modifiers":1048576,"shortcut":"⌘→","id":"22"},{"group":"Capture One","label":"Previous Photo","keyCode":123,"modifiers":1048576,"shortcut":"⌘←","id":"23"},{"group":"Capture One","label":"Undo","keyCode":6,"modifiers":1048576,"shortcut":"⌘Z","id":"24"},{"group":"Capture One","label":"Redo","keyCode":6,"modifiers":1179648,"shortcut":"⇧⌘Z","id":"25"},{"group":"Capture One","label":"Zoom Window","keyCode":6,"modifiers":1310720,"shortcut":"⌃⌘Z","id":"26"},{"group":"Capture One","label":"speedEdit · counterclockwise","keyCode":125,"modifiers":0,"shortcut":"↓","id":"27"},{"group":"Capture One","label":"speedEdit · clockwise","keyCode":126,"modifiers":0,"shortcut":"↑","id":"28"},{"group":"Capture One","label":"selectPhoto · counterclockwise","keyCode":123,"modifiers":0,"shortcut":"←","id":"29"},{"group":"Capture One","label":"selectPhoto · clockwise","keyCode":124,"modifiers":0,"shortcut":"→","id":"30"},{"group":"Chrome","label":"New Window","keyCode":45,"modifiers":1048576,"shortcut":"⌘N","id":"31"},{"group":"Chrome","label":"New Incognito Window","keyCode":45,"modifiers":1179648,"shortcut":"⇧⌘N","id":"32"},{"group":"Chrome","label":"New Tab","keyCode":17,"modifiers":1048576,"shortcut":"⌘T","id":"33"},{"group":"Chrome","label":"Reopen Closed Tab","keyCode":17,"modifiers":1179648,"shortcut":"⇧⌘T","id":"34"},{"group":"Chrome","label":"Next Tab","keyCode":48,"modifiers":262144,"shortcut":"⌃Tab","id":"35"},{"group":"Chrome","label":"Previous Tab","keyCode":48,"modifiers":393216,"shortcut":"⌃⇧Tab","id":"36"},{"group":"Chrome","label":"First Tab","keyCode":18,"modifiers":1048576,"shortcut":"⌘1","id":"37"},{"group":"Chrome","label":"Second Tab","keyCode":19,"modifiers":1048576,"shortcut":"⌘2","id":"38"},{"group":"Chrome","label":"Third Tab","keyCode":20,"modifiers":1048576,"shortcut":"⌘3","id":"39"},{"group":"Chrome","label":"Fourth Tab","keyCode":21,"modifiers":1048576,"shortcut":"⌘4","id":"40"},{"group":"Chrome","label":"Fifth Tab","keyCode":23,"modifiers":1048576,"shortcut":"⌘5","id":"41"},{"group":"Chrome","label":"Sixth Tab","keyCode":22,"modifiers":1048576,"shortcut":"⌘6","id":"42"},{"group":"Chrome","label":"Seventh Tab","keyCode":26,"modifiers":1048576,"shortcut":"⌘7","id":"43"},{"group":"Chrome","label":"Eighth Tab","keyCode":28,"modifiers":1048576,"shortcut":"⌘8","id":"44"},{"group":"Chrome","label":"Last Tab","keyCode":25,"modifiers":1048576,"shortcut":"⌘9","id":"45"},{"group":"Chrome","label":"Back","keyCode":33,"modifiers":1048576,"shortcut":"⌘[","id":"46"},{"group":"Chrome","label":"Forward","keyCode":30,"modifiers":1048576,"shortcut":"⌘]","id":"47"},{"group":"Chrome","label":"Close Tab","keyCode":13,"modifiers":1048576,"shortcut":"⌘W","id":"48"},{"group":"Chrome","label":"Close Window","keyCode":13,"modifiers":1179648,"shortcut":"⇧⌘W","id":"49"},{"group":"Chrome","label":"Minimize Chrome","keyCode":46,"modifiers":1048576,"shortcut":"⌘M","id":"50"},{"group":"Chrome","label":"Hide Chrome","keyCode":4,"modifiers":1048576,"shortcut":"⌘H","id":"51"},{"group":"Chrome","label":"Quit Chrome","keyCode":12,"modifiers":1048576,"shortcut":"⌘Q","id":"52"},{"group":"Chrome","label":"Show/Hide Bookmarks Bar","keyCode":11,"modifiers":1179648,"shortcut":"⇧⌘B","id":"53"},{"group":"Chrome","label":"Bookmark Manager","keyCode":11,"modifiers":1572864,"shortcut":"⌥⌘B","id":"54"},{"group":"Chrome","label":"Settings Page","keyCode":43,"modifiers":1048576,"shortcut":"⌘,","id":"55"},{"group":"Chrome","label":"History Page","keyCode":16,"modifiers":1048576,"shortcut":"⌘Y","id":"56"},{"group":"Chrome","label":"Downloads Page","keyCode":38,"modifiers":1179648,"shortcut":"⇧⌘J","id":"57"},{"group":"Chrome","label":"Find Text","keyCode":3,"modifiers":1048576,"shortcut":"⌘F","id":"58"},{"group":"Chrome","label":"Jump to Next Find Bar Result","keyCode":5,"modifiers":1048576,"shortcut":"⌘G","id":"59"},{"group":"Chrome","label":"Jump to Previous Find Bar Result","keyCode":5,"modifiers":1179648,"shortcut":"⇧⌘G","id":"60"},{"group":"Chrome","label":"Dev Tools","keyCode":34,"modifiers":1572864,"shortcut":"⌥⌘I","id":"61"},{"group":"Chrome","label":"Clear Browsing Data Options","keyCode":51,"modifiers":1179648,"shortcut":"⇧⌘Delete","id":"62"},{"group":"Chrome","label":"Jump to Address Bar","keyCode":37,"modifiers":1048576,"shortcut":"⌘L","id":"63"},{"group":"Chrome","label":"Search","keyCode":3,"modifiers":1572864,"shortcut":"⌥⌘F","id":"64"},{"group":"Chrome","label":"Print","keyCode":35,"modifiers":1048576,"shortcut":"⌘P","id":"65"},{"group":"Chrome","label":"Save","keyCode":1,"modifiers":1048576,"shortcut":"⌘S","id":"66"},{"group":"Chrome","label":"Page Setup","keyCode":35,"modifiers":1572864,"shortcut":"⌥⌘P","id":"67"},{"group":"Chrome","label":"Email Current Page","keyCode":34,"modifiers":1179648,"shortcut":"⇧⌘I","id":"68"},{"group":"Chrome","label":"Reload","keyCode":15,"modifiers":1048576,"shortcut":"⌘R","id":"69"},{"group":"Chrome","label":"Reload Without Caching","keyCode":15,"modifiers":1179648,"shortcut":"⇧⌘R","id":"70"},{"group":"Chrome","label":"Navigate to Next Clickable Item","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"71"},{"group":"Chrome","label":"Navigate to Previous Clickable Item","keyCode":48,"modifiers":131072,"shortcut":"⇧Tab","id":"72"},{"group":"Chrome","label":"Open File","keyCode":31,"modifiers":1048576,"shortcut":"⌘O","id":"73"},{"group":"Chrome","label":"Display HTML Source Code","keyCode":32,"modifiers":1572864,"shortcut":"⌥⌘U","id":"74"},{"group":"Chrome","label":"Open JavaScript Console","keyCode":38,"modifiers":1572864,"shortcut":"⌥⌘J","id":"75"},{"group":"Chrome","label":"Bookmark Page","keyCode":2,"modifiers":1048576,"shortcut":"⌘D","id":"76"},{"group":"Chrome","label":"Bookmark all Tabs","keyCode":2,"modifiers":1179648,"shortcut":"⇧⌘D","id":"77"},{"group":"Chrome","label":"Toggle Full-Screen","keyCode":3,"modifiers":1310720,"shortcut":"⌃⌘F","id":"78"},{"group":"Chrome","label":"Zoom In","keyCode":24,"modifiers":1048576,"shortcut":"⌘=","id":"79"},{"group":"Chrome","label":"Zoom Out","keyCode":27,"modifiers":1048576,"shortcut":"⌘-","id":"80"},{"group":"Chrome","label":"Reset Zoom (Actual Size)","keyCode":29,"modifiers":1048576,"shortcut":"⌘0","id":"81"},{"group":"Chrome","label":"Scroll Down by One Screen","keyCode":121,"modifiers":0,"shortcut":"Page Down","id":"82"},{"group":"Chrome","label":"Scroll Up by One Screen","keyCode":116,"modifiers":0,"shortcut":"Page Up","id":"83"},{"group":"Chrome","label":"Cursor to Beginning of Word","keyCode":123,"modifiers":524288,"shortcut":"⌥←","id":"84"},{"group":"Chrome","label":"Cursor to End of Word","keyCode":124,"modifiers":524288,"shortcut":"⌥→","id":"85"},{"group":"Chrome","label":"Delete Previous Word","keyCode":51,"modifiers":524288,"shortcut":"⌥Delete","id":"86"},{"group":"Chrome","label":"Open Home Page","keyCode":4,"modifiers":1179648,"shortcut":"⇧⌘H","id":"87"},{"group":"Chrome","label":"Switch Tabs · press","keyCode":13,"modifiers":1048576,"shortcut":"⌘W","id":"88"},{"group":"Chrome","label":"Switch Tabs · counterclockwise","keyCode":48,"modifiers":393216,"shortcut":"⌃⇧Tab","id":"89"},{"group":"Chrome","label":"Switch Tabs · clockwise","keyCode":48,"modifiers":262144,"shortcut":"⌃Tab","id":"90"},{"group":"Chrome","label":"Back/Forward · press","keyCode":15,"modifiers":1048576,"shortcut":"⌘R","id":"91"},{"group":"Chrome","label":"Back/Forward · counterclockwise","keyCode":123,"modifiers":1048576,"shortcut":"⌘←","id":"92"},{"group":"Chrome","label":"Back/Forward · clockwise","keyCode":124,"modifiers":1048576,"shortcut":"⌘→","id":"93"},{"group":"Chrome","label":"Zoom · press","keyCode":29,"modifiers":1048576,"shortcut":"⌘0","id":"94"},{"group":"Chrome","label":"Zoom · counterclockwise","keyCode":27,"modifiers":1048576,"shortcut":"⌘-","id":"95"},{"group":"Chrome","label":"Zoom · clockwise","keyCode":24,"modifiers":1048576,"shortcut":"⌘=","id":"96"},{"group":"Chrome","label":"Scroll Page · press","keyCode":126,"modifiers":1048576,"shortcut":"⌘↑","id":"97"},{"group":"Chrome","label":"Scroll Page · counterclockwise","keyCode":116,"modifiers":0,"shortcut":"Page Up","id":"98"},{"group":"Chrome","label":"Scroll Page · clockwise","keyCode":121,"modifiers":0,"shortcut":"Page Down","id":"99"},{"group":"Chrome","label":"Tab Through Page Elements · press","keyCode":36,"modifiers":0,"shortcut":"Return","id":"100"},{"group":"Chrome","label":"Tab Through Page Elements · counterclockwise","keyCode":48,"modifiers":131072,"shortcut":"⇧Tab","id":"101"},{"group":"Chrome","label":"Tab Through Page Elements · clockwise","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"102"},{"group":"Chrome","label":"Cycle Through Find Bar Results · press","keyCode":3,"modifiers":1048576,"shortcut":"⌘F","id":"103"},{"group":"Chrome","label":"Cycle Through Find Bar Results · counterclockwise","keyCode":5,"modifiers":1179648,"shortcut":"⇧⌘G","id":"104"},{"group":"Chrome","label":"Cycle Through Find Bar Results · clockwise","keyCode":5,"modifiers":1048576,"shortcut":"⌘G","id":"105"},{"group":"Safari","label":"New Window","keyCode":45,"modifiers":1048576,"shortcut":"⌘N","id":"106"},{"group":"Safari","label":"New Incognito Window","keyCode":45,"modifiers":1179648,"shortcut":"⇧⌘N","id":"107"},{"group":"Safari","label":"New Tab","keyCode":17,"modifiers":1048576,"shortcut":"⌘T","id":"108"},{"group":"Safari","label":"Reopen Closed Tab","keyCode":17,"modifiers":1179648,"shortcut":"⇧⌘T","id":"109"},{"group":"Safari","label":"Next Tab","keyCode":48,"modifiers":262144,"shortcut":"⌃Tab","id":"110"},{"group":"Safari","label":"Previous Tab","keyCode":48,"modifiers":393216,"shortcut":"⌃⇧Tab","id":"111"},{"group":"Safari","label":"First Tab","keyCode":18,"modifiers":1048576,"shortcut":"⌘1","id":"112"},{"group":"Safari","label":"Second Tab","keyCode":19,"modifiers":1048576,"shortcut":"⌘2","id":"113"},{"group":"Safari","label":"Third Tab","keyCode":20,"modifiers":1048576,"shortcut":"⌘3","id":"114"},{"group":"Safari","label":"Fourth Tab","keyCode":21,"modifiers":1048576,"shortcut":"⌘4","id":"115"},{"group":"Safari","label":"Fifth Tab","keyCode":23,"modifiers":1048576,"shortcut":"⌘5","id":"116"},{"group":"Safari","label":"Sixth Tab","keyCode":22,"modifiers":1048576,"shortcut":"⌘6","id":"117"},{"group":"Safari","label":"Seventh Tab","keyCode":26,"modifiers":1048576,"shortcut":"⌘7","id":"118"},{"group":"Safari","label":"Eighth Tab","keyCode":28,"modifiers":1048576,"shortcut":"⌘8","id":"119"},{"group":"Safari","label":"Last Tab","keyCode":25,"modifiers":1048576,"shortcut":"⌘9","id":"120"},{"group":"Safari","label":"Back","keyCode":33,"modifiers":1048576,"shortcut":"⌘[","id":"121"},{"group":"Safari","label":"Forward","keyCode":30,"modifiers":1048576,"shortcut":"⌘]","id":"122"},{"group":"Safari","label":"Close Tab","keyCode":13,"modifiers":1048576,"shortcut":"⌘W","id":"123"},{"group":"Safari","label":"Close Window","keyCode":13,"modifiers":1179648,"shortcut":"⇧⌘W","id":"124"},{"group":"Safari","label":"Minimize Safari","keyCode":46,"modifiers":1048576,"shortcut":"⌘M","id":"125"},{"group":"Safari","label":"Hide Safari","keyCode":4,"modifiers":1048576,"shortcut":"⌘H","id":"126"},{"group":"Safari","label":"Quit Safari","keyCode":12,"modifiers":1048576,"shortcut":"⌘Q","id":"127"},{"group":"Safari","label":"Show/Hide Bookmarks Bar","keyCode":11,"modifiers":1179648,"shortcut":"⇧⌘B","id":"128"},{"group":"Safari","label":"Bookmark Manager","keyCode":11,"modifiers":1572864,"shortcut":"⌥⌘B","id":"129"},{"group":"Safari","label":"Find Text","keyCode":3,"modifiers":1048576,"shortcut":"⌘F","id":"130"},{"group":"Safari","label":"Jump to Next Find Bar Result","keyCode":5,"modifiers":1048576,"shortcut":"⌘G","id":"131"},{"group":"Safari","label":"Jump to Previous Find Bar Result","keyCode":5,"modifiers":1179648,"shortcut":"⇧⌘G","id":"132"},{"group":"Safari","label":"Jump to Address Bar","keyCode":37,"modifiers":1048576,"shortcut":"⌘L","id":"133"},{"group":"Safari","label":"Search","keyCode":3,"modifiers":1572864,"shortcut":"⌥⌘F","id":"134"},{"group":"Safari","label":"Print","keyCode":35,"modifiers":1048576,"shortcut":"⌘P","id":"135"},{"group":"Safari","label":"Save","keyCode":1,"modifiers":1048576,"shortcut":"⌘S","id":"136"},{"group":"Safari","label":"Reload","keyCode":15,"modifiers":1048576,"shortcut":"⌘R","id":"137"},{"group":"Safari","label":"Reload from Origin","keyCode":15,"modifiers":1179648,"shortcut":"⇧⌘R","id":"138"},{"group":"Safari","label":"Navigate to Next Clickable Item","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"139"},{"group":"Safari","label":"Navigate to Previous Clickable Item","keyCode":48,"modifiers":131072,"shortcut":"⇧Tab","id":"140"},{"group":"Safari","label":"Open File","keyCode":31,"modifiers":1048576,"shortcut":"⌘O","id":"141"},{"group":"Safari","label":"Bookmark Page","keyCode":2,"modifiers":1048576,"shortcut":"⌘D","id":"142"},{"group":"Safari","label":"Bookmark all Tabs","keyCode":2,"modifiers":1179648,"shortcut":"⇧⌘D","id":"143"},{"group":"Safari","label":"Toggle Full-Screen","keyCode":3,"modifiers":1310720,"shortcut":"⌃⌘F","id":"144"},{"group":"Safari","label":"Zoom In","keyCode":24,"modifiers":1048576,"shortcut":"⌘=","id":"145"},{"group":"Safari","label":"Zoom Out","keyCode":27,"modifiers":1048576,"shortcut":"⌘-","id":"146"},{"group":"Safari","label":"Reset Zoom (Actual Size)","keyCode":29,"modifiers":1048576,"shortcut":"⌘0","id":"147"},{"group":"Safari","label":"Scroll Down by One Screen","keyCode":121,"modifiers":0,"shortcut":"Page Down","id":"148"},{"group":"Safari","label":"Scroll Up by One Screen","keyCode":116,"modifiers":0,"shortcut":"Page Up","id":"149"},{"group":"Safari","label":"Cursor to Beginning of Word","keyCode":123,"modifiers":524288,"shortcut":"⌥←","id":"150"},{"group":"Safari","label":"Cursor to End of Word","keyCode":124,"modifiers":524288,"shortcut":"⌥→","id":"151"},{"group":"Safari","label":"Delete Previous Word","keyCode":51,"modifiers":524288,"shortcut":"⌥Delete","id":"152"},{"group":"Safari","label":"Open Home Page","keyCode":4,"modifiers":1179648,"shortcut":"⇧⌘H","id":"153"},{"group":"Safari","label":"Switch Tabs · press","keyCode":13,"modifiers":1048576,"shortcut":"⌘W","id":"154"},{"group":"Safari","label":"Switch Tabs · counterclockwise","keyCode":48,"modifiers":393216,"shortcut":"⌃⇧Tab","id":"155"},{"group":"Safari","label":"Switch Tabs · clockwise","keyCode":48,"modifiers":262144,"shortcut":"⌃Tab","id":"156"},{"group":"Safari","label":"Back/Forward · press","keyCode":15,"modifiers":1048576,"shortcut":"⌘R","id":"157"},{"group":"Safari","label":"Back/Forward · counterclockwise","keyCode":123,"modifiers":1048576,"shortcut":"⌘←","id":"158"},{"group":"Safari","label":"Back/Forward · clockwise","keyCode":124,"modifiers":1048576,"shortcut":"⌘→","id":"159"},{"group":"Safari","label":"Zoom · press","keyCode":29,"modifiers":1048576,"shortcut":"⌘0","id":"160"},{"group":"Safari","label":"Zoom · counterclockwise","keyCode":27,"modifiers":1048576,"shortcut":"⌘-","id":"161"},{"group":"Safari","label":"Zoom · clockwise","keyCode":24,"modifiers":1048576,"shortcut":"⌘=","id":"162"},{"group":"Safari","label":"Scroll Page · press","keyCode":126,"modifiers":1048576,"shortcut":"⌘↑","id":"163"},{"group":"Safari","label":"Scroll Page · counterclockwise","keyCode":116,"modifiers":0,"shortcut":"Page Up","id":"164"},{"group":"Safari","label":"Scroll Page · clockwise","keyCode":121,"modifiers":0,"shortcut":"Page Down","id":"165"},{"group":"Safari","label":"Tab Through Page Elements · press","keyCode":36,"modifiers":0,"shortcut":"Return","id":"166"},{"group":"Safari","label":"Tab Through Page Elements · counterclockwise","keyCode":48,"modifiers":131072,"shortcut":"⇧Tab","id":"167"},{"group":"Safari","label":"Tab Through Page Elements · clockwise","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"168"},{"group":"Safari","label":"Cycle Through Find Bar Results · press","keyCode":3,"modifiers":1048576,"shortcut":"⌘F","id":"169"},{"group":"Safari","label":"Cycle Through Find Bar Results · counterclockwise","keyCode":5,"modifiers":1179648,"shortcut":"⇧⌘G","id":"170"},{"group":"Safari","label":"Cycle Through Find Bar Results · clockwise","keyCode":5,"modifiers":1048576,"shortcut":"⌘G","id":"171"},{"group":"Microsoft Teams","label":"toggle-mute-video-combo · press","keyCode":46,"modifiers":1179648,"shortcut":"⇧⌘M","id":"172"},{"group":"Microsoft Teams","label":"toggle-mute","keyCode":46,"modifiers":1179648,"shortcut":"⇧⌘M","id":"173"},{"group":"Microsoft Teams","label":"temp-mute-mac","keyCode":49,"modifiers":524288,"shortcut":"⌥Space","id":"174"},{"group":"Microsoft Teams","label":"toggle-video","keyCode":31,"modifiers":1179648,"shortcut":"⇧⌘O","id":"175"},{"group":"Microsoft Teams","label":"end-call","keyCode":4,"modifiers":1179648,"shortcut":"⇧⌘H","id":"176"},{"group":"Microsoft Teams","label":"start-stop-screenshare","keyCode":14,"modifiers":1179648,"shortcut":"⇧⌘E","id":"177"},{"group":"Microsoft Teams","label":"raise-lower-hand","keyCode":40,"modifiers":1179648,"shortcut":"⇧⌘K","id":"178"},{"group":"Microsoft Teams","label":"admit-people","keyCode":16,"modifiers":1179648,"shortcut":"⇧⌘Y","id":"179"},{"group":"Zoom","label":"toggle-mute-video-combo-mac · press","keyCode":0,"modifiers":1179648,"shortcut":"⇧⌘A","id":"180"},{"group":"Zoom","label":"toggle-mute-mac","keyCode":0,"modifiers":1179648,"shortcut":"⇧⌘A","id":"181"},{"group":"Zoom","label":"toggle-video-mac","keyCode":9,"modifiers":1179648,"shortcut":"⇧⌘V","id":"182"},{"group":"Zoom","label":"end-call-mac","keyCode":13,"modifiers":1048576,"shortcut":"⌘W","id":"183"},{"group":"Zoom","label":"start-stop-screenshare-mac","keyCode":1,"modifiers":1179648,"shortcut":"⇧⌘S","id":"184"},{"group":"Zoom","label":"pause-resume-screenshare-mac","keyCode":17,"modifiers":1179648,"shortcut":"⇧⌘T","id":"185"},{"group":"Illustrator","label":"Artboard tool","keyCode":31,"modifiers":131072,"shortcut":"⇧O","id":"186"},{"group":"Illustrator","label":"Selection tool","keyCode":9,"modifiers":0,"shortcut":"V","id":"187"},{"group":"Illustrator","label":"Direct Selection tool","keyCode":0,"modifiers":0,"shortcut":"A","id":"188"},{"group":"Illustrator","label":"Magic Wand tool","keyCode":16,"modifiers":0,"shortcut":"Y","id":"189"},{"group":"Illustrator","label":"Lasso tool","keyCode":12,"modifiers":0,"shortcut":"Q","id":"190"},{"group":"Illustrator","label":"Pen tool","keyCode":35,"modifiers":0,"shortcut":"P","id":"191"},{"group":"Illustrator","label":"Navigation.Pan & Zoom · counterclockwise","keyCode":27,"modifiers":1048576,"shortcut":"⌘-","id":"192"},{"group":"Illustrator","label":"Navigation.Pan & Zoom · clockwise","keyCode":24,"modifiers":1048576,"shortcut":"⌘=","id":"193"},{"group":"Lightroom","label":"Edit In Photoshop","keyCode":14,"modifiers":1048576,"shortcut":"⌘E","id":"194"},{"group":"Lightroom","label":"Group Into Stack","keyCode":5,"modifiers":1048576,"shortcut":"⌘G","id":"195"},{"group":"Lightroom","label":"Unstack","keyCode":5,"modifiers":1179648,"shortcut":"⇧⌘G","id":"196"},{"group":"Lightroom","label":"Toggle Stack","keyCode":1,"modifiers":0,"shortcut":"S","id":"197"},{"group":"Lightroom","label":"Move To Top Of Stack","keyCode":1,"modifiers":131072,"shortcut":"⇧S","id":"198"},{"group":"Lightroom","label":"Convert To Black & White","keyCode":9,"modifiers":0,"shortcut":"V","id":"199"},{"group":"Lightroom","label":"Auto Tone","keyCode":32,"modifiers":1048576,"shortcut":"⌘U","id":"200"},{"group":"Lightroom","label":"Auto White Balance","keyCode":32,"modifiers":1179648,"shortcut":"⇧⌘U","id":"201"},{"group":"Lightroom","label":"Copy Develop Settings","keyCode":8,"modifiers":1179648,"shortcut":"⇧⌘C","id":"202"},{"group":"Lightroom","label":"Paste Develop Settings","keyCode":9,"modifiers":1179648,"shortcut":"⇧⌘V","id":"203"},{"group":"Lightroom","label":"Paste Settings From Previous Photo","keyCode":9,"modifiers":1572864,"shortcut":"⌥⌘V","id":"204"},{"group":"Lightroom","label":"Delete Photo","keyCode":117,"modifiers":0,"shortcut":"Forward Delete","id":"205"},{"group":"Lightroom","label":"Delete Photo from Catalog","keyCode":117,"modifiers":524288,"shortcut":"⌥Forward Delete","id":"206"},{"group":"Lightroom","label":"Delete All Rejected Photos","keyCode":117,"modifiers":1048576,"shortcut":"⌘Forward Delete","id":"207"},{"group":"Lightroom","label":"Copy After To Before","keyCode":123,"modifiers":1703936,"shortcut":"⌥⇧⌘←","id":"208"},{"group":"Lightroom","label":"Copy Before To After","keyCode":124,"modifiers":1703936,"shortcut":"⌥⇧⌘→","id":"209"},{"group":"Lightroom","label":"Swap Before & After","keyCode":126,"modifiers":1703936,"shortcut":"⌥⇧⌘↑","id":"210"},{"group":"Lightroom","label":"Sync Settings","keyCode":1,"modifiers":1179648,"shortcut":"⇧⌘S","id":"211"},{"group":"Lightroom","label":"Create Snapshot","keyCode":45,"modifiers":1048576,"shortcut":"⌘N","id":"212"},{"group":"Lightroom","label":"Zoom In","keyCode":69,"modifiers":1048576,"shortcut":"⌘+ (keypad)","id":"213"},{"group":"Lightroom","label":"Zoom Out","keyCode":78,"modifiers":1048576,"shortcut":"⌘− (keypad)","id":"214"},{"group":"Lightroom","label":"Cycle Screen Modes","keyCode":3,"modifiers":131072,"shortcut":"⇧F","id":"215"},{"group":"Lightroom","label":"Toggle Image Fullscreen","keyCode":3,"modifiers":0,"shortcut":"F","id":"216"},{"group":"Lightroom","label":"Toggle Panels Fullscreen","keyCode":3,"modifiers":1179648,"shortcut":"⇧⌘F","id":"217"},{"group":"Lightroom","label":"Go Back (Previous Module)","keyCode":123,"modifiers":1572864,"shortcut":"⌥⌘←","id":"218"},{"group":"Lightroom","label":"Go Forward (Next Module)","keyCode":124,"modifiers":1572864,"shortcut":"⌥⌘→","id":"219"},{"group":"Lightroom","label":"Local Adjustments.Brush Size · counterclockwise","keyCode":33,"modifiers":0,"shortcut":"[","id":"220"},{"group":"Lightroom","label":"Local Adjustments.Brush Size · clockwise","keyCode":30,"modifiers":0,"shortcut":"]","id":"221"},{"group":"Lightroom","label":"Local Adjustments.Brush Feathering · counterclockwise","keyCode":33,"modifiers":131072,"shortcut":"⇧[","id":"222"},{"group":"Lightroom","label":"Local Adjustments.Brush Feathering · clockwise","keyCode":30,"modifiers":131072,"shortcut":"⇧]","id":"223"},{"group":"Lightroom","label":"Adjust Any Slider.Adjust Any Slider · counterclockwise","keyCode":125,"modifiers":0,"shortcut":"↓","id":"224"},{"group":"Lightroom","label":"Adjust Any Slider.Adjust Any Slider · clockwise","keyCode":126,"modifiers":0,"shortcut":"↑","id":"225"},{"group":"Lightroom","label":"Adjust Any Slider.Adjust Any Slider (Coarse) · counterclockwise","keyCode":125,"modifiers":131072,"shortcut":"⇧↓","id":"226"},{"group":"Lightroom","label":"Adjust Any Slider.Adjust Any Slider (Coarse) · clockwise","keyCode":126,"modifiers":131072,"shortcut":"⇧↑","id":"227"},{"group":"Lightroom","label":"Adjust Any Slider.Adjust Basic Panel Slider · press","keyCode":6,"modifiers":1048576,"shortcut":"⌘Z","id":"228"},{"group":"Lightroom","label":"Adjust Any Slider.Adjust Basic Panel Slider · counterclockwise","keyCode":78,"modifiers":0,"shortcut":"− (keypad)","id":"229"},{"group":"Lightroom","label":"Adjust Any Slider.Adjust Basic Panel Slider · clockwise","keyCode":69,"modifiers":0,"shortcut":"+ (keypad)","id":"230"},{"group":"Lightroom","label":"Adjust Any Slider.Select Basic Panel Slider · counterclockwise","keyCode":43,"modifiers":0,"shortcut":",","id":"231"},{"group":"Lightroom","label":"Adjust Any Slider.Select Basic Panel Slider · clockwise","keyCode":47,"modifiers":0,"shortcut":".","id":"232"},{"group":"Lightroom","label":"Photo.Rotate Photo 90º · counterclockwise","keyCode":33,"modifiers":1048576,"shortcut":"⌘[","id":"233"},{"group":"Lightroom","label":"Photo.Rotate Photo 90º · clockwise","keyCode":30,"modifiers":1048576,"shortcut":"⌘]","id":"234"},{"group":"Lightroom","label":"Photo.Move Up & Down in Stack · press","keyCode":1,"modifiers":131072,"shortcut":"⇧S","id":"235"},{"group":"Lightroom","label":"Photo.Move Up & Down in Stack · counterclockwise","keyCode":33,"modifiers":131072,"shortcut":"⇧[","id":"236"},{"group":"Lightroom","label":"Photo.Move Up & Down in Stack · clockwise","keyCode":30,"modifiers":131072,"shortcut":"⇧]","id":"237"},{"group":"Lightroom","label":"View.Grid Size · counterclockwise","keyCode":78,"modifiers":0,"shortcut":"− (keypad)","id":"238"},{"group":"Lightroom","label":"View.Grid Size · clockwise","keyCode":69,"modifiers":0,"shortcut":"+ (keypad)","id":"239"},{"group":"Lightroom","label":"View.Zoom · counterclockwise","keyCode":78,"modifiers":1048576,"shortcut":"⌘− (keypad)","id":"240"},{"group":"Lightroom","label":"View.Zoom · clockwise","keyCode":69,"modifiers":1048576,"shortcut":"⌘+ (keypad)","id":"241"},{"group":"Photoshop","label":"Open Image Size","keyCode":34,"modifiers":1572864,"shortcut":"⌥⌘I","id":"242"},{"group":"Photoshop","label":"Open Canvas Size","keyCode":8,"modifiers":1572864,"shortcut":"⌥⌘C","id":"243"},{"group":"Photoshop","label":"Select Previous Layer (Up)","keyCode":30,"modifiers":524288,"shortcut":"⌥]","id":"244"},{"group":"Photoshop","label":"Select Next Layer (Down)","keyCode":33,"modifiers":524288,"shortcut":"⌥[","id":"245"},{"group":"Photoshop","label":"Toggle Layer Visibility","keyCode":43,"modifiers":1048576,"shortcut":"⌘,","id":"246"},{"group":"Photoshop","label":"Move Layer Up","keyCode":30,"modifiers":1048576,"shortcut":"⌘]","id":"247"},{"group":"Photoshop","label":"Move Layer Down","keyCode":33,"modifiers":1048576,"shortcut":"⌘[","id":"248"},{"group":"Photoshop","label":"Copy Merged","keyCode":8,"modifiers":1179648,"shortcut":"⇧⌘C","id":"249"},{"group":"Photoshop","label":"Stamp Visible Layers","keyCode":14,"modifiers":1703936,"shortcut":"⌥⇧⌘E","id":"250"},{"group":"Photoshop","label":"Select All Layers","keyCode":0,"modifiers":1572864,"shortcut":"⌥⌘A","id":"251"},{"group":"Photoshop","label":"Toggle Clipping Mask","keyCode":5,"modifiers":1572864,"shortcut":"⌥⌘G","id":"252"},{"group":"Photoshop","label":"Fill Dialog","keyCode":96,"modifiers":131072,"shortcut":"⇧F5","id":"253"},{"group":"Photoshop","label":"Fill Foreground","keyCode":51,"modifiers":524288,"shortcut":"⌥Delete","id":"254"},{"group":"Photoshop","label":"Fill Background","keyCode":51,"modifiers":1048576,"shortcut":"⌘Delete","id":"255"},{"group":"Photoshop","label":"Reselect","keyCode":2,"modifiers":1179648,"shortcut":"⇧⌘D","id":"256"},{"group":"Photoshop","label":"Deselect","keyCode":2,"modifiers":1048576,"shortcut":"⌘D","id":"257"},{"group":"Photoshop","label":"Invert Selection","keyCode":34,"modifiers":1048576,"shortcut":"⌘I","id":"258"},{"group":"Photoshop","label":"(Hold) Hand Tool","keyCode":49,"modifiers":0,"shortcut":"Space","id":"259"},{"group":"Photoshop","label":"(Hold) Rotate Tool","keyCode":15,"modifiers":0,"shortcut":"R","id":"260"},{"group":"Photoshop","label":"Auto Leading","keyCode":0,"modifiers":1703936,"shortcut":"⌥⇧⌘A","id":"261"},{"group":"Photoshop","label":"Reset Tracking","keyCode":12,"modifiers":1048576,"shortcut":"⌘Q","id":"262"},{"group":"Photoshop","label":"Zoom to Fit","keyCode":29,"modifiers":1048576,"shortcut":"⌘0","id":"263"},{"group":"Photoshop","label":"Zoom to 100%","keyCode":18,"modifiers":1048576,"shortcut":"⌘1","id":"264"},{"group":"Photoshop","label":"Save For Web","keyCode":1,"modifiers":1703936,"shortcut":"⌥⇧⌘S","id":"265"},{"group":"Photoshop","label":"Show/Hide Panels","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"266"},{"group":"Photoshop","label":"Toggle Airbrush","keyCode":35,"modifiers":655360,"shortcut":"⌥⇧P","id":"267"},{"group":"Photoshop","label":"Character Size · press","keyCode":40,"modifiers":1179648,"shortcut":"⇧⌘K","id":"268"},{"group":"Photoshop","label":"Character Size · counterclockwise","keyCode":43,"modifiers":1179648,"shortcut":"⇧⌘,","id":"269"},{"group":"Photoshop","label":"Character Size · clockwise","keyCode":47,"modifiers":1179648,"shortcut":"⇧⌘.","id":"270"},{"group":"Photoshop","label":"Leading · press","keyCode":0,"modifiers":1179648,"shortcut":"⇧⌘A","id":"271"},{"group":"Photoshop","label":"Leading · counterclockwise","keyCode":126,"modifiers":524288,"shortcut":"⌥↑","id":"272"},{"group":"Photoshop","label":"Leading · clockwise","keyCode":125,"modifiers":524288,"shortcut":"⌥↓","id":"273"},{"group":"Photoshop","label":"Kerning/Tracking · press","keyCode":12,"modifiers":1048576,"shortcut":"⌘Q","id":"274"},{"group":"Photoshop","label":"Kerning/Tracking · counterclockwise","keyCode":123,"modifiers":524288,"shortcut":"⌥←","id":"275"},{"group":"Photoshop","label":"Kerning/Tracking · clockwise","keyCode":124,"modifiers":524288,"shortcut":"⌥→","id":"276"},{"group":"Photoshop","label":"Baseline Shift · press","keyCode":4,"modifiers":1179648,"shortcut":"⇧⌘H","id":"277"},{"group":"Photoshop","label":"Baseline Shift · counterclockwise","keyCode":125,"modifiers":1703936,"shortcut":"⌥⇧⌘↓","id":"278"},{"group":"Photoshop","label":"Baseline Shift · clockwise","keyCode":126,"modifiers":1703936,"shortcut":"⌥⇧⌘↑","id":"279"},{"group":"Photoshop","label":"Adjust Any property (select number field) · press","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"280"},{"group":"Photoshop","label":"Adjust Any property (select number field) · counterclockwise","keyCode":125,"modifiers":0,"shortcut":"↓","id":"281"},{"group":"Photoshop","label":"Adjust Any property (select number field) · clockwise","keyCode":126,"modifiers":0,"shortcut":"↑","id":"282"},{"group":"Photoshop","label":"Adjust Any property (large) · press","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"283"},{"group":"Photoshop","label":"Adjust Any property (large) · counterclockwise","keyCode":125,"modifiers":131072,"shortcut":"⇧↓","id":"284"},{"group":"Photoshop","label":"Adjust Any property (large) · clockwise","keyCode":126,"modifiers":131072,"shortcut":"⇧↑","id":"285"},{"group":"Photoshop","label":"Adjust Any Property.Select Property · press","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"286"},{"group":"Photoshop","label":"Adjust Any Property.Select Property · counterclockwise","keyCode":48,"modifiers":131072,"shortcut":"⇧Tab","id":"287"},{"group":"Photoshop","label":"Adjust Any Property.Select Property · clockwise","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"288"},{"group":"Photoshop","label":"Brush Size/Airbrush · press","keyCode":35,"modifiers":655360,"shortcut":"⌥⇧P","id":"289"},{"group":"Photoshop","label":"Brush Size/Airbrush · counterclockwise","keyCode":33,"modifiers":0,"shortcut":"[","id":"290"},{"group":"Photoshop","label":"Brush Size/Airbrush · clockwise","keyCode":30,"modifiers":0,"shortcut":"]","id":"291"},{"group":"Photoshop","label":"Brush Hardness/Crosshair · counterclockwise","keyCode":33,"modifiers":131072,"shortcut":"⇧[","id":"292"},{"group":"Photoshop","label":"Brush Hardness/Crosshair · clockwise","keyCode":30,"modifiers":131072,"shortcut":"⇧]","id":"293"},{"group":"Premiere Pro","label":"Transport.Select Clip at Playhead","keyCode":2,"modifiers":0,"shortcut":"D","id":"294"},{"group":"Premiere Pro","label":"Sequence.Add Edit Point","keyCode":40,"modifiers":1048576,"shortcut":"⌘K","id":"295"},{"group":"Premiere Pro","label":"Sequence.Trim Edit","keyCode":17,"modifiers":131072,"shortcut":"⇧T","id":"296"},{"group":"Premiere Pro","label":"Sequence.Select Next Sequence","keyCode":20,"modifiers":131072,"shortcut":"⇧3","id":"297"},{"group":"Premiere Pro","label":"Adjust Any Property.Adjust Any Property (fine) · press","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"298"},{"group":"Premiere Pro","label":"Adjust Any Property.Adjust Any Property (fine) · counterclockwise","keyCode":125,"modifiers":0,"shortcut":"↓","id":"299"},{"group":"Premiere Pro","label":"Adjust Any Property.Adjust Any Property (fine) · clockwise","keyCode":126,"modifiers":0,"shortcut":"↑","id":"300"},{"group":"Premiere Pro","label":"Adjust Any Property.Adjust Any Property (coarse) · press","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"301"},{"group":"Premiere Pro","label":"Adjust Any Property.Adjust Any Property (coarse) · counterclockwise","keyCode":125,"modifiers":131072,"shortcut":"⇧↓","id":"302"},{"group":"Premiere Pro","label":"Adjust Any Property.Adjust Any Property (coarse) · clockwise","keyCode":126,"modifiers":131072,"shortcut":"⇧↑","id":"303"},{"group":"Premiere Pro","label":"Adjust Any Property.Select Property · press","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"304"},{"group":"Premiere Pro","label":"Adjust Any Property.Select Property · counterclockwise","keyCode":48,"modifiers":131072,"shortcut":"⇧Tab","id":"305"},{"group":"Premiere Pro","label":"Adjust Any Property.Select Property · clockwise","keyCode":48,"modifiers":0,"shortcut":"Tab","id":"306"}]
"""#
}

struct AppProfile: Codable, Identifiable, Equatable {
    var id = UUID().uuidString
    var name: String
    var bundleID: String
    var appPath: String
    var display = "both"
    var customIconPNG: Data? = nil
    var mappings: [String: Mapping] = [:]
    var ledColors: [String:String] = [:]
}
struct ProfileBackup: Codable {
    var formatVersion = 1
    var profiles: [AppProfile]
    var unitNames: [String:String]
    var sliderFlipped: [String:Bool]? = nil
    var whiteBalance: [String:[Double]]? = nil
    var consoleOrientation: Int? = nil
}

struct Module: Identifiable, Equatable {
    var id: String
    var index: Int
    var kind: Int
    var name: String
    var controls: [(String, String)] {
        switch kind {
        case 25: return (1...2).map { ("pressed_\($0)", "Button \($0)") }
        case 8: return (1...3).map { ("pressed_\($0)", "Button \($0)") }
        case 10: return (1...3).map { ("set_\($0)", "Slider \($0)") }
        case 9: return (1...3).flatMap { n in [("rotation_right_\(n)", "Knob \(n) · clockwise"), ("rotation_left_\(n)", "Knob \(n) · counterclockwise"), ("pressed_\(n)", "Knob \(n) · press")] }
        case 24: return [("rotation_right", "Ring · clockwise"), ("rotation_left", "Ring · counterclockwise"), ("joystick_x", "Pad · direction 1"), ("joystick_y", "Pad · direction 2")]
        default: return []
        }
    }
}
struct ControlChoice: Equatable {
    var uid: String
    var event: String
    var title: String
    var key: String { uid + "|" + event }
}

func profileName(_ url: URL) -> String {
    let name=url.deletingPathExtension().lastPathComponent
    if name=="Digital Performer",let version=Bundle(url:url)?.object(forInfoDictionaryKey:"CFBundleShortVersionString") as? String {return name+" ("+version+")"}
    return name
}
func sameAppPath(_ first:String,_ second:String)->Bool {
    URL(fileURLWithPath:first).resolvingSymlinksInPath().standardizedFileURL.path == URL(fileURLWithPath:second).resolvingSymlinksInPath().standardizedFileURL.path
}

func appIcon(_ path: String) -> NSImage? {
    if let bundle=Bundle(path:path),let name=bundle.object(forInfoDictionaryKey:"CFBundleIconFile") as? String {
        let filename=(name as NSString).pathExtension.isEmpty ? name+".icns":name
        if let image=NSImage(contentsOfFile:path+"/Contents/Resources/"+filename),image.isValid {return image}
    }
    guard !path.isEmpty else{return nil}
    let image=NSWorkspace.shared.icon(forFile:path)
    return image.isValid ? image:nil
}

func profileIcon(_ profile: AppProfile) -> NSImage? {
    if let data=profile.customIconPNG,let image=NSImage(data:data),image.isValid {return image}
    return appIcon(profile.appPath)
}

func makeScreen(name: String, icon: NSImage?, mode: String) -> (Data, NSImage) {
    let image = NSImage(size: NSSize(width: 240, height: 240))
    image.lockFocus()
    NSColor(calibratedRed: 0.035, green: 0.05, blue: 0.08, alpha: 1).setFill()
    NSBezierPath(rect: NSRect(x: 0, y: 0, width: 240, height: 240)).fill()
    if mode != "name", let icon = icon {
        let size: CGFloat = mode == "icon" ? 180 : 124
        icon.draw(in: NSRect(x: (240-size)/2, y: mode == "icon" ? 30 : 88, width: size, height: size), from: .zero, operation: .sourceOver, fraction: 1)
    }
    if mode != "icon" || icon == nil {
        let para = NSMutableParagraphStyle(); para.alignment = .center; para.lineBreakMode = .byWordWrapping
        let fontSize: CGFloat = name.count > 21 ? 20 : 25
        let attrs: [NSAttributedString.Key: Any] = [.font: NSFont.systemFont(ofSize: fontSize, weight: .semibold), .foregroundColor: NSColor.white, .paragraphStyle: para]
        (name as NSString).draw(in: NSRect(x: 12, y: mode == "name" ? 65 : 12, width: 216, height: mode == "name" ? 110 : 66), withAttributes: attrs)
    }
    image.unlockFocus()
    let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 240, pixelsHigh: 240, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 960, bitsPerPixel: 32)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
    image.draw(in: NSRect(x: 0,y: 0,width: 240,height: 240))
    NSGraphicsContext.restoreGraphicsState()
    var data = Data(capacity: 115200)
    for y in 0..<240 { for x in 0..<240 {
        let c = bitmap.colorAt(x: x, y: y)!.usingColorSpace(.deviceRGB)!
        let r = Int((c.redComponent * 255).rounded()), g = Int((c.greenComponent * 255).rounded()), b = Int((c.blueComponent * 255).rounded())
        let pixel = UInt16(((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3))
        data.append(UInt8(pixel >> 8)); data.append(UInt8(pixel & 255))
    }}
    return (data, image)
}

final class Studio: ObservableObject {
    @Published var profiles: [AppProfile] = []
    @Published var selectedProfile = "" {
        didSet {if !selectedProfile.isEmpty {try? selectedProfile.write(to:storage.deletingLastPathComponent().appendingPathComponent("last-edited-profile.txt"),atomically:true,encoding:.utf8)}}
    }
    @Published var selectedControl: ControlChoice?
    @Published var modules: [Module] = []
    @Published var connection = "Disconnected"
    @Published var startAtLogin = false
    @Published var startupNeedsApproval = false
    @Published var connected = false
    @Published var activeName = "Finder"
    @Published var activeBundle = "com.apple.finder"
    @Published var activePath = "/System/Library/CoreServices/Finder.app"
    @Published var learning = false
    @Published var recordingShortcut = false
    @Published var lastInput = "Touch a control to see its activity here."
    @Published var message = ""
    @Published var screenPreview: NSImage?
    @Published var actionsEnabled = false {
        didSet {try? JSONEncoder().encode(["actionsEnabled":actionsEnabled]).write(to:storage.deletingLastPathComponent().appendingPathComponent("preferences.json"),options:.atomic)}
    }
    @Published var screenMode = "both"
    @Published var access = AXIsProcessTrusted()
    @Published var showLog = false
    @Published var showPermissions = false
    @Published var logText = "No connection yet. Click Connect controller. Close any BBC inspection before connecting."
    @Published var unitNames: [String:String] = [:]
    @Published var sliderFlipped: [String:Bool] = [:]
    @Published var whiteBalance: [String:[Double]] = [:]
    @Published var ledTestHex: String?
    @Published var ledTestName = ""
    @Published var previewProfileID: String?
    @Published var identifyingUID: String?
    @Published var calibratingPad = false
    @Published var consoleOrientation = 0
    @Published var resetModuleUID: String?
    @Published var confirmResetModule = false
    @Published var copySource: String?
    @Published var confirmCopy = false
    private var calibrationProfile: String?
    private var pageLatched: [String:Bool] = [:]
    private var diagnosticTimes: [String:Double] = [:]
    private var persistenceBlocked = false
    private var identifyTimer: Timer?
    private var identifyOn = false
    private var wantsConnection = false
    private var reconnectAt = Date.distantPast
    private var connectionStarted = Date.distantPast
    private var process: Process?
    private var input: FileHandle?
    private var timer: Timer?
    private var observer: NSObjectProtocol?
    private var monitor: Any?
    private var lastActions: [String: Double] = [:]
    private var previousValues: [String: Double] = [:]
    private var accumulated: [String: Double] = [:]
    private var learnedAt = Date.distantPast
    private var lastFrame = Data()
    private var errorLog = ""
    private var appObserver: NSObjectProtocol?
    let storage: URL
    let frameURL: URL
    let logURL: URL
    init(profileHome: URL? = nil) {
        let root = profileHome ?? ProcessInfo.processInfo.environment["MONOGRAM_PROFILE_HOME"].map { URL(fileURLWithPath:$0,isDirectory:true) } ?? FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("MONOcc", isDirectory: true)
        storage = root.appendingPathComponent("profiles.json")
        frameURL = root.appendingPathComponent("active-screen.rgb565")
        logURL = root.appendingPathComponent("connection.log")
        try? FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let legacy = FileManager.default.urls(for:.applicationSupportDirectory,in:.userDomainMask)[0].appendingPathComponent("Monogram Profiles/profiles.json")
        let stored = (try? Data(contentsOf: storage)) ?? (ProcessInfo.processInfo.environment["MONOGRAM_PROFILE_HOME"] == nil ? try? Data(contentsOf:legacy):nil)
        if let data = stored {
            do { profiles = try JSONDecoder().decode([AppProfile].self, from: data); save() }
            catch { persistenceBlocked=true;message = "Saved profiles could not be read. The original file is preserved; saving is paused until a backup is restored." }
        } else { profiles = defaults(); save() }
        for candidate in defaults() where !profiles.contains(where:{sameAppPath($0.appPath,candidate.appPath)}) {profiles.append(candidate)}
        save()
        if let data=try? Data(contentsOf:root.appendingPathComponent("units.json")),let names=try? JSONDecoder().decode([String:String].self,from:data) {unitNames=names}
        if let data=try? Data(contentsOf:root.appendingPathComponent("preferences.json")),let preferences=try? JSONDecoder().decode([String:Bool].self,from:data) {actionsEnabled=preferences["actionsEnabled"] ?? false}
        if let data=try? Data(contentsOf:root.appendingPathComponent("slider-labels.json")),let values=try? JSONDecoder().decode([String:Bool].self,from:data) {sliderFlipped=values}
        if let data=try? Data(contentsOf:root.appendingPathComponent("white-balance.json")),let values=try? JSONDecoder().decode([String:[Double]].self,from:data) {whiteBalance=validWhiteBalance(values)}
        if let data=try? Data(contentsOf:root.appendingPathComponent("console-orientation.json")),let degrees=try? JSONDecoder().decode(Int.self,from:data),[0,90,180,270].contains(degrees) {consoleOrientation=degrees}
        repairPadBindings()
        let lastEdited=try? String(contentsOf:root.appendingPathComponent("last-edited-profile.txt"),encoding:.utf8)
        let populated=profiles.max{a,b in a.mappings.values.filter{$0.action != "none"}.count < b.mappings.values.filter{$0.action != "none"}.count}
        selectedProfile = profiles.first(where:{$0.id==lastEdited})?.id ?? (populated?.mappings.values.contains(where:{$0.action != "none"})==true ? populated?.id:nil) ?? profiles.first?.id ?? ""
        observer = NSWorkspace.shared.notificationCenter.addObserver(forName: NSWorkspace.didActivateApplicationNotification, object: nil, queue: .main) { [weak self] n in
            if let app = n.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication { self?.activate(app) }
        }
        if let app = NSWorkspace.shared.frontmostApplication { activate(app) }
        timer = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { [weak self] _ in
            guard let self = self else { return }
            self.access = AXIsProcessTrusted()
            if self.showPermissions {self.refreshLoginStatus()}
            if self.connected { self.send(["layout":true]) }
            if self.shouldReconnect(at:Date()) {self.connect()}
            if self.wantsConnection,!self.connected,self.process != nil,Date().timeIntervalSince(self.connectionStarted)>20 {self.process?.terminate()}
        }
        appObserver = NotificationCenter.default.addObserver(forName: NSApplication.willTerminateNotification, object: nil, queue: .main) { [weak self] _ in self?.disconnect() }
        refreshScreen()
    }
    var loginService:SMAppService {SMAppService.loginItem(identifier:"org.monocc.login")}
    func refreshLoginStatus() {
        startAtLogin=loginService.status == .enabled
        startupNeedsApproval=loginService.status == .requiresApproval
    }
    func configureStartAtLogin(_ enabled:Bool) {
        guard ProcessInfo.processInfo.environment["MONOGRAM_PROFILE_HOME"] == nil else {return}
        do {
            if enabled {try loginService.register()} else {try loginService.unregister()}
            refreshLoginStatus()
            try JSONEncoder().encode(true).write(to:storage.deletingLastPathComponent().appendingPathComponent("login-configured.json"),options:.atomic)
            message=startupNeedsApproval ? "Allow MONOcc in System Settings → General → Login Items to start it automatically.":(startAtLogin ? "MONOcc will start in the background when you log in.":"Automatic login startup is off. MONOcc keeps running until you stop it.")
        } catch {refreshLoginStatus();message="Could not change automatic startup: \(error.localizedDescription)"}
    }
    func prepareBackgroundStartup() {
        guard ProcessInfo.processInfo.environment["MONOGRAM_PROFILE_HOME"] == nil else {return}
        refreshLoginStatus()
        let marker=storage.deletingLastPathComponent().appendingPathComponent("login-configured.json")
        if !FileManager.default.fileExists(atPath:marker.path),Bundle.main.bundleURL.path.hasPrefix("/Applications/") {configureStartAtLogin(true)}
    }
    func finishSetup() {
        learning=false;calibratingPad=false;recordingShortcut=false;showLog=false;showPermissions=false
        if let monitor=monitor {NSEvent.removeMonitor(monitor);self.monitor=nil}
        identifyTimer?.invalidate();identifyTimer=nil;identifyingUID=nil;identifyOn=false
        ledTestHex=nil;previewProfileID=nil
        applyLEDs()
    }
    func shouldReconnect(at date:Date)->Bool {wantsConnection && process == nil && date >= reconnectAt}
    func defaults() -> [AppProfile] {
        var found = [AppProfile(name: "Finder", bundleID: "com.apple.finder", appPath: "/System/Library/CoreServices/Finder.app")]
        let wanted = ["digital performer", "affinity photo", "affinity designer", "affinity publisher", "duckduckgo"]
        var urls: [URL] = []
        for root in ["/Applications", NSHomeDirectory()+"/Applications"] {
            if let e = FileManager.default.enumerator(at: URL(fileURLWithPath: root), includingPropertiesForKeys: nil, options: [.skipsHiddenFiles]) {
                while let url = e.nextObject() as? URL {
                    if url.pathExtension == "app" { urls.append(url); e.skipDescendants() }
                    else if url.pathComponents.count > 6 { e.skipDescendants() }
                }
            }
        }
        for url in urls.sorted(by: {$0.path < $1.path}) {
            let name = url.deletingPathExtension().lastPathComponent
            guard wanted.contains(where: {name.lowercased().contains($0)}), let id = Bundle(url: url)?.bundleIdentifier, !id.isEmpty else { continue }
            if !found.contains(where:{$0.appPath==url.path}) {
                found.append(AppProfile(name:profileName(url),bundleID:id,appPath:url.path))
            }
        }
        return found
    }
    var activeProfile: AppProfile? {
        if let exact=profiles.first(where:{$0.bundleID==activeBundle && sameAppPath($0.appPath,activePath)}) {return exact}
        let matches=profiles.filter{$0.bundleID==activeBundle}
        return matches.count==1 ? matches.first:nil
    }
    func controls(for module:Module)->[(String,String)] {
        let controls=module.controls
        return module.kind==10 && sliderFlipped[module.id,default:true] ? Array(controls.reversed()):controls
    }
    func swapSliderLabels(_ uid:String,_ flipped:Bool) {
        sliderFlipped[uid]=flipped
        do {try JSONEncoder().encode(sliderFlipped).write(to:storage.deletingLastPathComponent().appendingPathComponent("slider-labels.json"),options:.atomic)}
        catch {message="Could not save this unit’s label order."}
    }
    var colorStatus:String {
        if ledTestHex != nil {return "Testing all lights: \(ledTestName)"}
        if let id=previewProfileID,let profile=profiles.first(where:{$0.id==id}) {return "Preview: \(profile.name)"}
        if let profile=activeProfile {return "Following app: \(profile.name)"}
        return "\(activeName): default white (no profile)"
    }
    func previewColors() {
        guard editing != nil else{return}
        ledTestHex=nil;previewProfileID=selectedProfile;applyLEDs()
        message="Previewing \(editing?.name ?? "this profile") colors. They remain saved for that app; switching apps ends this preview."
    }
    func endColorPreview() {ledTestHex=nil;previewProfileID=nil;applyLEDs()}
    func testLights(_ hex:String,name:String) {
        guard connected,hex.count==6,UInt32(hex,radix:16) != nil else{return}
        identifyTimer?.invalidate();identifyingUID=nil;identifyOn=false
        ledTestHex=hex;ledTestName=name;previewProfileID=nil
        applyLEDs()
        appendLog("Requested LED test: \(name), RGB #\(hex), \(ledPlan().count) lights across \(modules.count) units")
        message="All lights: \(name). Test stays on when switching apps. Choose End light test to restore saved colors."
    }
    func unitTitle(_ module:Module)->String {unitNames[module.id].flatMap{$0.isEmpty ? nil:$0} ?? "\(module.name) · Unit \(module.index)"}
    func renameUnit(_ uid:String,_ name:String) {
        unitNames[uid]=name
        do {try JSONEncoder().encode(unitNames).write(to:storage.deletingLastPathComponent().appendingPathComponent("units.json"),options:.atomic)}
        catch {message="Could not save the unit name."}
    }
    func identify(_ module:Module) {
        guard connected else{return}
        identifyTimer?.invalidate();identifyingUID=module.id;identifyOn=true;applyLEDs()
        var remaining=7
        identifyTimer=Timer.scheduledTimer(withTimeInterval:0.35,repeats:true){[weak self] timer in
            guard let self=self else{timer.invalidate();return}
            remaining-=1
            if remaining<=0 {timer.invalidate();self.identifyingUID=nil;self.identifyOn=false;self.applyLEDs()}
            else {self.identifyOn.toggle();self.applyLEDs()}
        }
    }
    func appendLog(_ line:String) {
        logText += "\n"+line
        if logText.count>24000 {logText=String(logText.suffix(24000))}
        try? logText.write(to:logURL,atomically:true,encoding:.utf8)
    }
    var editing: AppProfile? { profiles.first {$0.id == selectedProfile} }
    var mapping: Mapping {
        guard let key = selectedControl?.key else { return Mapping() }
        return editing?.mappings[key] ?? Mapping()
    }
    func save() {
        guard !persistenceBlocked else{return}
        do { let e = JSONEncoder(); e.outputFormatting = [.prettyPrinted, .sortedKeys]; try e.encode(profiles).write(to: storage, options: .atomic) }
        catch { message = "Could not save profiles: \(error.localizedDescription)" }
    }
    func update(_ transform: (inout Mapping) -> Void) {
        guard let i = profiles.firstIndex(where: {$0.id == selectedProfile}), let key = selectedControl?.key else { return }
        var m = profiles[i].mappings[key] ?? Mapping(); transform(&m)
        if m.action=="pages",let c=selectedControl,c.event.hasPrefix("joystick") {
            for event in ["joystick_x","joystick_y"] where event != c.event && profiles[i].mappings[c.uid+"|"+event]?.action=="pages" {profiles[i].mappings.removeValue(forKey:c.uid+"|"+event)}
        }
        profiles[i].mappings[key] = m; save()
    }
    func applyPreset(_ preset:ShortcutPreset) {
        guard let control=selectedControl, control.event.hasPrefix("pressed_") || control.event.hasPrefix("rotation") else{return}
        update { $0=preset.mapping }
        message="Saved \(preset.label) for \(editing?.name ?? "this app"). Other assignments and colors were kept."
    }
    func updateDisplay(_ mode: String) {
        guard let i = profiles.firstIndex(where: {$0.id == selectedProfile}) else { return }
        profiles[i].display = mode; save(); refreshScreen()
    }
    func persistOrientation(_ degrees:Int) throws {
        try JSONEncoder().encode(degrees).write(to:storage.deletingLastPathComponent().appendingPathComponent("console-orientation.json"),options:.atomic)
    }
    func rotateConsole(to degrees:Int) {
        guard [0,90,180,270].contains(degrees) else{return}
        do {
            try persistOrientation(degrees);consoleOrientation=degrees
            previousValues.removeAll();accumulated.removeAll();pageLatched.removeAll()
            send(["orientation":degrees])
            message="Console rotation saved: \(degrees)°. Recalibrate Orbiter pad page direction if you turn your physical console."
        } catch {message="Could not save console rotation: \(error.localizedDescription)"}
    }
    func setCustomIcon(_ data:Data?) {
        guard !persistenceBlocked,let i=profiles.firstIndex(where:{$0.id==selectedProfile}) else{return}
        if let data=data {guard data.count<1_000_000,let image=NSImage(data:data),image.isValid else {message="Choose a valid image smaller than 1 MB after resizing.";return}}
        var updated=profiles;updated[i].customIconPNG=data
        do {
            try JSONEncoder().encode(updated).write(to:storage,options:.atomic)
            profiles=updated;refreshScreen()
            message=data == nil ? "Original program icon restored for \(profiles[i].name).":"Custom Core screen icon saved for \(profiles[i].name)."
        } catch {message="Could not save the icon: \(error.localizedDescription)"}
    }
    func chooseCustomIcon() {
        let panel=NSOpenPanel();panel.title="Choose a Core screen icon for \(editing?.name ?? "this profile")";panel.allowedContentTypes=[.png,.jpeg,.tiff,.heic];panel.canChooseDirectories=false
        guard panel.runModal() == .OK,let url=panel.url else{return}
        do {
            let values=try url.resourceValues(forKeys:[.fileSizeKey]);guard (values.fileSize ?? 0)<20_000_000,let image=NSImage(contentsOf:url),image.isValid else {message="Choose a readable image under 20 MB.";return}
            let bitmap=NSBitmapImageRep(bitmapDataPlanes:nil,pixelsWide:240,pixelsHigh:240,bitsPerSample:8,samplesPerPixel:4,hasAlpha:true,isPlanar:false,colorSpaceName:.deviceRGB,bytesPerRow:960,bitsPerPixel:32)!
            NSGraphicsContext.saveGraphicsState();NSGraphicsContext.current=NSGraphicsContext(bitmapImageRep:bitmap)
            NSColor.clear.setFill();NSBezierPath(rect:NSRect(x:0,y:0,width:240,height:240)).fill()
            let scale=min(240/image.size.width,240/image.size.height)
            let size=NSSize(width:image.size.width*scale,height:image.size.height*scale)
            image.draw(in:NSRect(x:(240-size.width)/2,y:(240-size.height)/2,width:size.width,height:size.height))
            NSGraphicsContext.restoreGraphicsState()
            if let png=bitmap.representation(using:.png,properties:[:]) {setCustomIcon(png)}
        } catch {message="Could not read the image: \(error.localizedDescription)"}
    }
    func resetModule(_ uid:String) {
        guard !persistenceBlocked,modules.contains(where:{$0.id==uid}),let i=profiles.firstIndex(where:{$0.id==selectedProfile}) else{return}
        do {
            let backup=storage.deletingLastPathComponent().appendingPathComponent("Before module reset \(UUID().uuidString).json")
            try JSONEncoder().encode(ProfileBackup(profiles:profiles,unitNames:unitNames,sliderFlipped:sliderFlipped,whiteBalance:whiteBalance,consoleOrientation:consoleOrientation)).write(to:backup,options:.atomic)
            var updated=profiles
            updated[i].mappings=updated[i].mappings.filter{!$0.key.hasPrefix(uid+"|")}
            updated[i].ledColors=updated[i].ledColors.filter{!$0.key.hasPrefix(uid+"|")}
            try JSONEncoder().encode(updated).write(to:storage,options:.atomic)
            profiles=updated;previousValues.removeAll();accumulated.removeAll();pageLatched.removeAll()
            endColorPreview()
            message="Reset this module’s assignments and colors in \(profiles[i].name). A backup was saved automatically."
        } catch {message="Could not reset this module: \(error.localizedDescription)"}
    }
    func copySettings(from sourceID:String) {
        guard let source=profiles.first(where:{$0.id==sourceID}),let target=profiles.firstIndex(where:{$0.id==selectedProfile}),sourceID != selectedProfile else{return}
        do {
            let backup=storage.deletingLastPathComponent().appendingPathComponent("Before profile copy \(Int(Date().timeIntervalSince1970)).json")
            try JSONEncoder().encode(ProfileBackup(profiles:profiles,unitNames:unitNames,sliderFlipped:sliderFlipped,whiteBalance:whiteBalance,consoleOrientation:consoleOrientation)).write(to:backup,options:.atomic)
            profiles[target].mappings=source.mappings
            profiles[target].ledColors=source.ledColors
            profiles[target].display=source.display
            profiles[target].customIconPNG=source.customIconPNG
            save();refreshScreen();applyLEDs()
            message="Copied settings from \(source.name) to \(profiles[target].name). These profiles can now be edited independently."
        } catch {message="Could not copy profile settings: \(error.localizedDescription)"}
    }
    func controlTitle(uid:String,event:String,fallback:String)->String {
        if modules.first(where:{$0.id==uid})?.kind==10,event.hasPrefix("set_"),let n=Int(event.split(separator:"_").last ?? "") {
            let position=sliderFlipped[uid,default:true] ? 4-n:n
            return [1:"Left slider",2:"Center slider",3:"Right slider"][position] ?? fallback
        }
        guard event.hasPrefix("joystick"),let profile=editing,let vertical=pageAxis(in:profile,uid:uid) else{return fallback}
        return uid+"|"+event==vertical ? "Pad up/down · Page Up/Down":"Pad left/right"
    }
    func pageAxis(in profile:AppProfile,uid:String)->String? {
        let candidates=profile.mappings.filter{key,m in key.hasPrefix(uid+"|joystick_") && m.action=="pages"}
        return candidates.first(where:{$0.value.label=="Pad up / down · one page"})?.key ?? candidates.keys.sorted().first
    }
    func repairPadBindings() {
        var repaired=profiles
        var changed=false
        for i in repaired.indices {
            let keys=repaired[i].mappings.keys.filter{$0.hasSuffix("|joystick_x") || $0.hasSuffix("|joystick_y")}
            let uids=Set(keys.compactMap{key -> String? in guard let split=key.lastIndex(of:"|") else{return nil};return String(key[..<split])})
            for uid in uids {
                if let keep=pageAxis(in:repaired[i],uid:uid) {
                    for key in keys where key.hasPrefix(uid+"|") && key != keep && repaired[i].mappings[key]?.action=="pages" {
                        repaired[i].mappings.removeValue(forKey:key);changed=true
                    }
                }
            }
        }
        if changed {
            let backup=storage.deletingLastPathComponent().appendingPathComponent("Before pad repair \(Int(Date().timeIntervalSince1970)).json")
            do {try JSONEncoder().encode(ProfileBackup(profiles:profiles,unitNames:unitNames,sliderFlipped:sliderFlipped,whiteBalance:whiteBalance,consoleOrientation:consoleOrientation)).write(to:backup,options:.atomic)}
            catch {message="Could not back up pad settings; automatic repair was skipped.";return}
            profiles=repaired;save();message="Removed a conflicting second page axis. Your calibrated pad direction and other settings were kept."
        }
    }
    func exportBackup() {
        let panel=NSSavePanel();panel.title="Export MONOcc backup";panel.allowedContentTypes=[.json]
        let formatter=DateFormatter();formatter.dateFormat="yyyy-MM-dd"
        panel.nameFieldStringValue="MONOcc Backup "+formatter.string(from:Date())+".json"
        guard panel.runModal() == .OK,let url=panel.url else{return}
        do {let encoder=JSONEncoder();encoder.outputFormatting=[.prettyPrinted,.sortedKeys];try encoder.encode(ProfileBackup(profiles:profiles,unitNames:unitNames,sliderFlipped:sliderFlipped,whiteBalance:whiteBalance,consoleOrientation:consoleOrientation)).write(to:url,options:.atomic);message="Backup saved to \(url.path)"}
        catch {message="Could not export backup: \(error.localizedDescription)"}
    }
    func restoreBackup() {
        let panel=NSOpenPanel();panel.title="Restore MONOcc backup";panel.allowedContentTypes=[.json];panel.canChooseDirectories=false
        guard panel.runModal() == .OK,let url=panel.url else{return}
        do {
            let data=try Data(contentsOf:url)
            guard data.count<5_000_000 else{message="This file is too large for a MONOcc backup.";return}
            let backup=try JSONDecoder().decode(ProfileBackup.self,from:data)
            guard backup.formatVersion==1 else{message="This backup uses an unsupported version.";return}
            // Keep a dated local copy of current settings before importing.
            let before=storage.deletingLastPathComponent().appendingPathComponent("Before restore \(Int(Date().timeIntervalSince1970)).json")
            if persistenceBlocked,let original=try? Data(contentsOf:storage) {try original.write(to:before,options:.atomic)}
            else {try JSONEncoder().encode(ProfileBackup(profiles:profiles,unitNames:unitNames,sliderFlipped:sliderFlipped,whiteBalance:whiteBalance,consoleOrientation:consoleOrientation)).write(to:before,options:.atomic)}
            persistenceBlocked=false
            for profile in backup.profiles {
                if let i=profiles.firstIndex(where:{$0.bundleID==profile.bundleID && sameAppPath($0.appPath,profile.appPath)}) {profiles[i]=profile}
                else {profiles.append(profile)}
            }
            unitNames.merge(backup.unitNames){_,imported in imported}
            if let labels=backup.sliderFlipped {sliderFlipped.merge(labels){_,imported in imported};try JSONEncoder().encode(sliderFlipped).write(to:storage.deletingLastPathComponent().appendingPathComponent("slider-labels.json"),options:.atomic)}
            try JSONEncoder().encode(unitNames).write(to:storage.deletingLastPathComponent().appendingPathComponent("units.json"),options:.atomic)
            if let values=backup.whiteBalance {
                let merged=whiteBalance.merging(validWhiteBalance(values)){_,new in new}
                try JSONEncoder().encode(merged).write(to:storage.deletingLastPathComponent().appendingPathComponent("white-balance.json"),options:.atomic)
                whiteBalance=merged
            }
            if let degrees=backup.consoleOrientation,[0,90,180,270].contains(degrees) {try persistOrientation(degrees);consoleOrientation=degrees;send(["orientation":degrees])}
            selectedProfile=profiles.first?.id ?? "";selectedControl=nil;save();refreshScreen();applyLEDs()
            message="Backup restored. The previous settings were kept beside your saved profiles."
        } catch {message="Could not restore this backup: \(error.localizedDescription)"}
    }
    func setReverse(_ reversed:Bool) {
        guard let c=selectedControl else{return}
        if c.event=="rotation_left" || c.event=="rotation_right",mapping.action=="scroll",let i=profiles.firstIndex(where:{$0.id==selectedProfile}) {
            for event in ["rotation_left","rotation_right"] {
                let key=c.uid+"|"+event
                if profiles[i].mappings[key]?.action=="scroll" {profiles[i].mappings[key]?.invert=reversed}
            }
            save()
        } else {update{$0.invert=reversed}}
    }
    func calibratePad() {
        guard connected else{return};calibratingPad=true;calibrationProfile=selectedProfile;learning=false
        message="Push the Orbiter pad UP once, then let it return to center. MONOcc will learn its orientation."
    }
    func pageThreshold(sensitivity:Double)->Double {
        let gain=sensitivity.isFinite ? min(4,max(0.25,sensitivity)):1
        return 15+15/gain
    }
    func releasePagePad(uid:String,x:Double,y:Double) {
        // Only the calibrated page axis determines release. Sideways resting drift
        // must not block the next up/down push. Separate thresholds prevent chatter.
        if let profile=activeProfile ?? editing,let axis=pageAxis(in:profile,uid:uid),let mapping=profile.mappings[axis] {
            let release=min(24,pageThreshold(sensitivity:mapping.sensitivity)*0.75)
            let value=axis.hasSuffix("joystick_x") ? x:y
            if abs(value)<=release {pageLatched[uid+"|pad"]=false}
        } else if max(abs(x),abs(y))<=22 {pageLatched[uid+"|pad"]=false}
    }
    func pageTrigger(key:String,value:Double,reverse:Bool,sensitivity:Double=1)->Bool? {
        guard !pageLatched[key,default:false],abs(value)>pageThreshold(sensitivity:sensitivity) else{return nil}
        pageLatched[key]=true
        return (reverse ? -value:value)<0
    }
    func sendPage(up:Bool) {
        guard let target=NSWorkspace.shared.frontmostApplication,target.bundleIdentifier==activeBundle else{return}
        let key:CGKeyCode=up ? 116:121
        let source=CGEventSource(stateID:.hidSystemState)
        for down in [true,false] {
            let event=CGEvent(keyboardEventSource:source,virtualKey:key,keyDown:down)
            event?.flags=[]
            event?.post(tap:.cghidEventTap)
        }
        appendLog("Sent \(up ? "Page Up":"Page Down") to \(target.localizedName ?? activeName)")
    }
    func addApp() {
        let panel = NSOpenPanel(); panel.title = "Choose an app for a new profile"; panel.allowedContentTypes = [.applicationBundle]; panel.canChooseDirectories = false; panel.directoryURL = URL(fileURLWithPath:"/Applications")
        guard panel.runModal() == .OK, let url = panel.url, let bundle = Bundle(url: url), let id = bundle.bundleIdentifier else { return }
        if let index = profiles.firstIndex(where: {sameAppPath($0.appPath,url.path)}) {
            // Different installed versions can have the same bundle identity.
            // Preserve mappings and LED choices while updating the selected app.
            profiles[index].name = profileName(url)
            profiles[index].appPath = url.path
            selectedProfile = profiles[index].id;save();refreshScreen()
            message="Profile updated to \(profiles[index].name). Your assignments and colors were kept."
            return
        }
        let p = AppProfile(name: profileName(url), bundleID:id, appPath:url.path)
        profiles.append(p); selectedProfile = p.id; selectedControl = nil; save()
    }
    func addActive() {
        if let p = profiles.first(where: {$0.bundleID == activeBundle && sameAppPath($0.appPath,activePath)}) { selectedProfile = p.id; return }
        let p = AppProfile(name:activeName,bundleID:activeBundle,appPath:activePath)
        profiles.append(p); selectedProfile=p.id; save()
    }
    func activate(_ app: NSRunningApplication) {
        guard let id = app.bundleIdentifier, id != Bundle.main.bundleIdentifier, id != "com.apple.loginwindow" else { return }
        let changed = id != activeBundle || !sameAppPath(app.bundleURL?.path ?? "",activePath)
        activeBundle = id; activeName = app.localizedName ?? "Application"; activePath = app.bundleURL?.path ?? ""
        if changed { previewProfileID=nil;lastActions.removeAll(); previousValues.removeAll(); accumulated.removeAll(); refreshScreen();applyLEDs() }
    }
    func refreshScreen() {
        let mode = profiles.first(where: {$0.bundleID == activeBundle && sameAppPath($0.appPath,activePath)})?.display ?? "both"
        let icon = activeProfile.map{profileIcon($0)} ?? appIcon(activePath)
        let (bytes, image) = makeScreen(name:activeProfile?.name ?? activeName,icon:icon,mode:mode)
        screenPreview=image
        guard connected, bytes != lastFrame else { return }
        do { try bytes.write(to:frameURL, options:.atomic); send(["frame":frameURL.path]); lastFrame=bytes }
        catch { message="Could not prepare the Core screen: \(error.localizedDescription)" }
    }
    func connect() {
        wantsConnection=true
        reconnectAt=Date().addingTimeInterval(5)
        guard process == nil else { return }
        guard let executable = Bundle.main.url(forResource:"sextant",withExtension:nil) else { message="The device helper is missing."; return }
        // Seed only missing resources; init preserves all existing user edits.
        let seed=Process();seed.executableURL=executable;seed.arguments=["init"]
        seed.standardOutput=FileHandle.nullDevice;seed.standardError=FileHandle.nullDevice
        do {try seed.run();seed.waitUntilExit();if seed.terminationStatus != 0 {message="Could not initialize the controller resources.";return}}
        catch {message="Could not initialize the controller: \(error.localizedDescription)";return}
        let p=Process(); p.executableURL=executable; p.arguments=["tools","profile-bridge"]
        if ProcessInfo.processInfo.arguments.contains("--simulator") { p.arguments=["--device","simulator","--fake-layout","monogram","tools","profile-bridge"] }
        let stdin=Pipe(), stdout=Pipe(), stderr=Pipe(); p.standardInput=stdin; p.standardOutput=stdout; p.standardError=stderr
        connectionStarted=Date()
        errorLog=""; logText="Connecting to the controller…"; connection="Connecting…"; process=p; input=stdin.fileHandleForWriting; lastFrame=Data()
        try? Data().write(to:logURL)
        p.terminationHandler = { [weak self] task in DispatchQueue.main.async {
            guard let self=self, self.process === task else { return }
            self.process=nil;self.input=nil;self.connected=false;self.modules=[];self.connection=self.wantsConnection ? "Waiting for controller…":"Disconnected";self.reconnectAt=Date().addingTimeInterval(5);self.appendLog("Device helper exited: \(task.terminationStatus)")
            if task.terminationStatus != 0 { self.message="Waiting for the controller. MONOcc will reconnect automatically. Close any BBC inspection if it is using the device." }
        }}
        do { try p.run();appendLog("MONOcc \(Bundle.main.object(forInfoDictionaryKey:"CFBundleShortVersionString") as? String ?? "test"); Accessibility=\(access); app=\(Bundle.main.bundlePath)");appendLog("Starting device helper…") }
        catch {process=nil;input=nil;connection="Disconnected";message=error.localizedDescription;appendLog(error.localizedDescription);return}
        DispatchQueue.global(qos:.userInitiated).async { [weak self] in
            var buffer=Data()
            while true {
                let chunk=stdout.fileHandleForReading.availableData
                if chunk.isEmpty { break };buffer.append(chunk)
                while let pos=buffer.firstIndex(of:10) {
                    let line=String(decoding:buffer[..<pos],as:UTF8.self);buffer.removeSubrange(...pos)
                    DispatchQueue.main.async { self?.receive(line) }
                }
            }
        }
        stderr.fileHandleForReading.readabilityHandler = { [weak self] h in
            let data=h.availableData;guard !data.isEmpty else { return }
            DispatchQueue.main.async {
                guard let self=self else { return }; self.errorLog += String(decoding:data,as:UTF8.self);self.appendLog(String(decoding:data,as:UTF8.self))
                if self.errorLog.count > 18000 { self.errorLog=String(self.errorLog.suffix(18000)) }
                
            }
        }

    }
    func disconnect() {
        wantsConnection=false
        send(["quit":true]); try? input?.close(); input=nil
        if let p=process { DispatchQueue.global().asyncAfter(deadline:.now()+3) { if p.isRunning { p.terminate() } } }
        connected=false;connection="Disconnected";modules=[];lastFrame=Data()
    }
    func send(_ object:[String:Any]) {
        guard let input=input, let data=try? JSONSerialization.data(withJSONObject:object) else { return }
        do { try input.write(contentsOf:data+Data([10])) } catch { message="The device connection closed." }
    }
    func receive(_ line:String) {
        if let bytes=line.data(using:.utf8), let obj=(try? JSONSerialization.jsonObject(with:bytes)) as? [String:Any] {
            if let list=obj["modules"] as? [[String:Any]] {
                let result=list.compactMap { m -> Module? in
                    guard let uid=m["uid"] as? String, let index=m["index"] as? Int, let kind=m["kind"] as? Int, let name=m["name"] as? String else { return nil }
                    return Module(id:uid,index:index,kind:kind,name:name)
                }
                if result != modules { modules=result;applyLEDs();appendLog("Detected \(result.count) units: \(result.map{$0.name}.joined(separator:", "))") }
                let was=connected;connected=true;connection="\(result.count) units connected"
                if !was {
                    send(["orientation":consoleOrientation]);refreshScreen();applyLEDs()
                    DispatchQueue.main.asyncAfter(deadline:.now()+0.9){[weak self] in self?.applyLEDs()}
                    DispatchQueue.main.asyncAfter(deadline:.now()+2){[weak self] in self?.applyLEDs()}
                }
            }
            if obj["disconnected"] as? Bool == true {connected=false;connection="Waiting for controller…";modules=[];lastFrame=Data();previousValues.removeAll();accumulated.removeAll();pageLatched.removeAll()}
            if obj["orientationApplied"] != nil {lastFrame=Data();refreshScreen();applyLEDs()}
            if let err=obj["error"] as? String { message=err;appendLog(err) }
            return
        }
        guard line.hasPrefix("[in] i="), let iEnd=line.firstIndex(of:" "), let rawStart=line.range(of:" raw="), let split=line.range(of:" => ") else { return }
        _ = iEnd; _ = rawStart
        let prefix=line.dropFirst(7);guard let number=prefix.split(separator:" ").first,let index=Int(number),let module=modules.first(where:{$0.index==index}) else{return}
        if module.kind==24,let start=line.range(of:"raw=["),let end=line[start.upperBound...].firstIndex(of:"]") {
            let values=line[start.upperBound..<end].split(separator:",").compactMap{Int($0)}
            if values.count==8 {
                func axis(_ hi:Int,_ lo:Int)->Int {let n=(hi<<8)|lo;return n>=32768 ? n-65536:n}
                let x=axis(values[1],values[2]),y=axis(values[3],values[4])
                releasePagePad(uid:module.id,x:Double(x),y:Double(y))
                if calibratingPad,max(abs(x),abs(y))>80,let i=profiles.firstIndex(where:{$0.id==calibrationProfile}) {
                    let event=abs(x)>abs(y) ? "joystick_x":"joystick_y",value=abs(x)>abs(y) ? x:y
                    // Remove a previous page binding on the other axis of this pad.
                    for other in ["joystick_x","joystick_y"] where profiles[i].mappings[module.id+"|"+other]?.action=="pages" {profiles[i].mappings.removeValue(forKey:module.id+"|"+other)}
                    var mapping=Mapping();mapping.action="pages";mapping.invert=value>0;mapping.label="Pad up / down · one page"
                    profiles[i].mappings[module.id+"|"+event]=mapping
                    selectedControl=ControlChoice(uid:module.id,event:event,title:"Pad · page up/down")
                    pageLatched[module.id+"|pad"]=true;calibratingPad=false;save()
                    message="Pad learned. Release to center, then switch to \(profiles[i].name). Each up/down push sends one Page Up/Down."
                    return
                }
            }
        }
        for token in line[split.upperBound...].split(separator:" ") {
            let parts=token.split(separator:"=");guard parts.count==2,let value=Double(parts[1]) else{continue}
            let event=String(parts[0]); let choice=ControlChoice(uid:module.id,event:event,title:"\(module.name) · \(event)")
            if event.hasPrefix("set") && module.kind != 10 { continue }
            if event.hasPrefix("released") { continue }
            lastInput="\(module.name) \(module.index) · \(event) = \(Int(value))"
            if learning {
                selectedControl=choice;learning=false;learnedAt=Date();return
            }
            if Date().timeIntervalSince(learnedAt)<0.8 {continue}
            dispatch(choice:choice,value:value,kind:module.kind)
        }
    }
    func ringScrolling() {
        guard let c=selectedControl,let i=profiles.firstIndex(where:{$0.id==selectedProfile}) else{return}
        for event in ["rotation_left","rotation_right"] {
            var m=profiles[i].mappings[c.uid+"|"+event] ?? Mapping()
            m.action="scroll";m.invert=true;m.label=event=="rotation_left" ? "Scroll up":"Scroll down"
            profiles[i].mappings[c.uid+"|"+event]=m
        }
        save();message="Ring scrolling saved: clockwise down, counterclockwise up. Switch to this app to use it."
    }
    func sendScroll(_ amount:Int32) {
        guard amount != 0,let target=NSWorkspace.shared.frontmostApplication, target.bundleIdentifier==activeBundle else{return}
        guard let event=CGEvent(scrollWheelEvent2Source:CGEventSource(stateID:.hidSystemState),units:.pixel,wheelCount:1,wheel1:amount,wheel2:0,wheel3:0) else{return}
        // Scroll in the target app's focused window instead of at the pointer,
        // which may still be over the editor or Dock after app switching.
        let app=AXUIElementCreateApplication(target.processIdentifier)
        var window:CFTypeRef?
        if AXUIElementCopyAttributeValue(app,kAXFocusedWindowAttribute as CFString,&window) == .success,let window=window {
            var position:CFTypeRef?,size:CFTypeRef?
            AXUIElementCopyAttributeValue(window as! AXUIElement,kAXPositionAttribute as CFString,&position)
            AXUIElementCopyAttributeValue(window as! AXUIElement,kAXSizeAttribute as CFString,&size)
            if let position=position,let size=size,CFGetTypeID(position)==AXValueGetTypeID(),CFGetTypeID(size)==AXValueGetTypeID() {
                var point=CGPoint.zero;var dimensions=CGSize.zero
                if AXValueGetValue(position as! AXValue,.cgPoint,&point),AXValueGetValue(size as! AXValue,.cgSize,&dimensions) {
                    event.location=CGPoint(x:point.x+dimensions.width/2,y:point.y+dimensions.height/2)
                }
            }
        }
        event.flags=[]
        event.post(tap:.cghidEventTap)
        logOccasionally("scroll", "Sent scroll \(amount) to \(target.localizedName ?? activeName)")
    }
    func logOccasionally(_ key:String,_ text:String) {
        let now=Date.timeIntervalSinceReferenceDate
        if now-(diagnosticTimes[key] ?? 0)>1.5 {diagnosticTimes[key]=now;appendLog(text)}
    }
    func dispatch(choice:ControlChoice,value:Double,kind:Int) {
        guard actionsEnabled else {logOccasionally("disabled","Input received; actions are OFF.");return}
        guard access else {logOccasionally("access","Input received; macOS reports Accessibility permission is unavailable.");return}
        guard !learning, !recordingShortcut, !calibratingPad, identifyingUID == nil else{return}
        guard NSWorkspace.shared.frontmostApplication?.bundleIdentifier == activeBundle else {logOccasionally("editing","Input received while editing; no action sent.");return}
        guard let profile=activeProfile else {logOccasionally("noProfile","No profile for active app: \(activeName)");return}
        guard let m=profile.mappings[choice.key],m.action != "none" else{return}
        if m.action=="pages" {
            guard pageAxis(in:profile,uid:choice.uid)==choice.key else{return}
            logOccasionally("pad", "Pad \(choice.event)=\(Int(value)); centered/re-armed=\(!pageLatched[choice.uid+"|pad",default:false])")
            if let up=pageTrigger(key:choice.uid+"|pad",value:value,reverse:m.invert,sensitivity:m.sensitivity) {sendPage(up:up)}
            return
        }
        let now=Date.timeIntervalSinceReferenceDate
        let prior=previousValues[choice.key];previousValues[choice.key]=value
        let absolute=kind==10
        // The first absolute frame only seeds pickup. It never triggers a shortcut.
        if absolute && prior == nil {return}
        let delta=absolute ? value-(prior ?? value) : value
        let signed=m.invert ? -delta : delta
        if m.action=="volume" {
            guard now-(lastActions[choice.key] ?? 0)>0.12 else{return};lastActions[choice.key]=now
            let volume=Int(max(0,min(100,(m.invert ? 255-value:value)/255*100)))
            guard absolute else{return}
            let script=NSAppleScript(source:"set volume output volume \(volume)")
            var error:NSDictionary?;script?.executeAndReturnError(&error)
            if error != nil {message="macOS could not change output volume."}
            return
        }
        var units=1
        if absolute || choice.event.hasPrefix("joystick") {
            if choice.event.hasPrefix("joystick") { guard abs(value)>30 else{return} }
            let threshold=absolute ? max(1,16/m.sensitivity) : 30.0
            accumulated[choice.key,default:0] += signed
            guard abs(accumulated[choice.key]!)>=threshold else{return}
            accumulated[choice.key]=0
        } else if choice.event.hasPrefix("rotation") { units=max(1,min(12,Int(abs(value)*m.sensitivity))) }
        guard now-(lastActions[choice.key] ?? 0)>0.07 else{return};lastActions[choice.key]=now
        if m.action=="scroll" {
            let scaled=signed*m.sensitivity*24
            let amount=Int32(max(-240,min(240,scaled)).rounded())
            sendScroll(amount)
        } else if m.action=="shortcut",!m.shortcut.isEmpty {
            for _ in 0..<units {
                for down in [true,false] {let e=CGEvent(keyboardEventSource:nil,virtualKey:m.keyCode,keyDown:down);e?.flags=CGEventFlags(rawValue:m.modifiers);e?.post(tap:.cghidEventTap)}
            }
        }
    }
    var ledKey: String? {
        guard let c=selectedControl, let m=modules.first(where:{$0.id==c.uid}) else{return nil}
        let n = [8,9,10].contains(m.kind) ? (Int(c.event.split(separator:"_").last ?? "1") ?? 1)-1 : 0
        return c.uid+"|"+String(max(0,min(2,n)))
    }
    var ledColor: Color {
        guard let key=ledKey,let hex=editing?.ledColors[key],let value=UInt32(hex,radix:16) else{return .white}
        return Color(red:Double((value>>16)&255)/255,green:Double((value>>8)&255)/255,blue:Double(value&255)/255)
    }
    func setLEDColor(_ color:Color) {
        guard let key=ledKey,let i=profiles.firstIndex(where:{$0.id==selectedProfile}),let c=NSColor(color).usingColorSpace(.deviceRGB) else{return}
        profiles[i].ledColors[key]=String(format:"%02X%02X%02X",Int((c.redComponent*255).rounded()),Int((c.greenComponent*255).rounded()),Int((c.blueComponent*255).rounded()))
        save();applyLEDs()
    }
    func validWhiteBalance(_ values:[String:[Double]])->[String:[Double]] {
        values.filter{$0.value.count==3 && $0.value.allSatisfy{$0.isFinite && (0...1).contains($0)}}
    }
    func whiteBalanceGain(_ channel:Int)->Double {
        guard let key=ledKey,let gains=whiteBalance[key],gains.count==3,(0..<3).contains(channel) else{return 1}
        return gains[channel]
    }
    func setWhiteBalanceGain(_ channel:Int,_ value:Double) {
        guard let key=ledKey,(0..<3).contains(channel),value.isFinite else{return}
        var values=whiteBalance;var gains=values[key] ?? [1,1,1]
        gains[channel]=max(0,min(1,value));values[key]=gains
        storeWhiteBalance(values)
    }
    func resetWhiteBalance() {
        guard let key=ledKey else{return};var values=whiteBalance;values.removeValue(forKey:key);storeWhiteBalance(values)
    }
    func storeWhiteBalance(_ values:[String:[Double]]) {
        do {
            try JSONEncoder().encode(values).write(to:storage.deletingLastPathComponent().appendingPathComponent("white-balance.json"),options:.atomic)
            whiteBalance=values;applyLEDs()
        } catch {message="Could not save white balance: \(error.localizedDescription)"}
    }
    func balancedRGB(_ rgb:[Int],key:String)->[Int] {
        let gains=whiteBalance[key] ?? [1,1,1]
        return zip(rgb,gains).map{Int((Double($0.0)*$0.1).rounded())}
    }
    func applyLEDs() {
        guard connected else{return}
        send(["leds":ledPlan(for:profiles.first(where:{$0.id==previewProfileID}))])
    }
    func ledPlan(for profile:AppProfile? = nil)->[[String:Any]] {
        var lights:[[String:Any]]=[]
        let colors=(profile ?? activeProfile)?.ledColors ?? [:]
        for module in modules {
            for sub in 0..<([8,9,10].contains(module.kind) ? 3:1) {
                let value=UInt32(ledTestHex ?? colors[module.id+"|"+String(sub)] ?? "FFFFFF",radix:16) ?? 0xFFFFFF
                lights.append(["uid":module.id,"sub":sub,"rgb":balancedRGB([Int((value>>16)&255),Int((value>>8)&255),Int(value&255)],key:module.id+"|"+String(sub))])
            }
        }
        if let uid=identifyingUID,let module=modules.first(where:{$0.id==uid}) {
            lights.removeAll{($0["uid"] as? String)==uid}
            for sub in 0..<([8,9,10].contains(module.kind) ? 3:1) {
                lights.append(["uid":uid,"sub":sub,"rgb":identifyOn ? [255,180,0]:[0,0,0]])
            }
        }
        return lights
    }

    func requestAccess() {
        let options=[kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String:true] as CFDictionary
        access=AXIsProcessTrustedWithOptions(options)
    }
    func captureShortcut() {
        guard selectedControl != nil else{return}; recordingShortcut=true
        if let monitor=monitor {NSEvent.removeMonitor(monitor)}
        monitor=NSEvent.addLocalMonitorForEvents(matching:.keyDown) { [weak self] event in
            guard let self=self else{return event}
            if event.keyCode != 53 {
                let flags=event.modifierFlags.intersection([.command,.option,.control,.shift])
                let symbols=(flags.contains(.control) ? "⌃":"")+(flags.contains(.option) ? "⌥":"")+(flags.contains(.shift) ? "⇧":"")+(flags.contains(.command) ? "⌘":"")
                let key=event.charactersIgnoringModifiers?.uppercased() ?? "Key \(event.keyCode)"
                self.update { m in m.action="shortcut";m.keyCode=event.keyCode;m.modifiers=UInt64(flags.rawValue);m.shortcut=symbols+key }
            }
            self.recordingShortcut=false
            if let monitor=self.monitor {NSEvent.removeMonitor(monitor);self.monitor=nil}
            return nil
        }
    }
    func finderStarter() {
        guard editing?.bundleID=="com.apple.finder", let m=modules.first(where:{$0.kind==8}), let i=profiles.firstIndex(where:{$0.id==selectedProfile}) else{return}
        let definitions:[(String,String,UInt16,String)]=[("pressed_1","New Finder window",45,"⌘N"),("pressed_2","Quick Look",49,"Space"),("pressed_3","New folder",45,"⇧⌘N")]
        for (event,label,key,shortcut) in definitions {
            let k=m.id+"|"+event
            if profiles[i].mappings[k] == nil {profiles[i].mappings[k]=Mapping(label:label,action:"shortcut",keyCode:key,modifiers:event=="pressed_2" ? 0 : UInt64(NSEvent.ModifierFlags.command.rawValue | (event=="pressed_3" ? NSEvent.ModifierFlags.shift.rawValue:0)),shortcut:shortcut)}
        }
        save();message="Finder starter saved on Button unit \(m.index): button 1 New window, button 2 Quick Look, button 3 New folder. Existing assignments were kept."
    }
}

struct MainView: View {
    @ObservedObject var studio:Studio
    func field<T>(_ get:@escaping (Mapping)->T,_ set:@escaping (inout Mapping,T)->Void)->Binding<T>{Binding(get:{get(studio.mapping)},set:{v in studio.update{set(&$0,v)}})}
    var body: some View {
        HStack(spacing:0) {
            VStack(alignment:.leading,spacing:18) {
                HStack(alignment:.firstTextBaseline,spacing:8) {
                    Text("MONOcc").font(.system(size:13,weight:.bold,design:.rounded)).tracking(3)
                    Text("v" + (Bundle.main.object(forInfoDictionaryKey:"CFBundleShortVersionString") as? String ?? "—")).font(.system(size:12)).monospacedDigit()
                }.foregroundColor(.secondary)
                Text("Your apps.\nYour controls.").font(.system(size:27,weight:.semibold)).lineSpacing(3)
                Button(action:studio.addApp){Label("Add an app",systemImage:"plus")}.buttonStyle(.borderedProminent)
                ScrollView { VStack(spacing:6) { ForEach(studio.profiles) { p in
                    Button {studio.selectedProfile=p.id;studio.selectedControl=nil} label:{
                        HStack {Image(nsImage:profileIcon(p) ?? NSImage(systemSymbolName:"app",accessibilityDescription:nil)!).resizable().frame(width:26,height:26);Text(p.name).fontWeight(studio.selectedProfile==p.id ? .semibold:.regular);Spacer();Text("\(p.mappings.values.filter{$0.action != "none"}.count)").font(.caption.monospacedDigit()).foregroundColor(.secondary);if p.id==studio.activeProfile?.id {Circle().fill(Color.green).frame(width:7,height:7)}}.padding(10).background(studio.selectedProfile==p.id ? Color.white.opacity(0.10):Color.clear).cornerRadius(9)
                    }.buttonStyle(.plain)
                }}}
                Spacer()
                Text(studio.connection).font(.callout).foregroundColor(studio.connected ? .green:.secondary)
                Button(studio.connected ? "Disconnect":"Connect controller") {studio.connected ? studio.disconnect():studio.connect()}
                Text("Runs in the background when setup is closed.").font(.caption).foregroundColor(.secondary)
                Toggle("Start at login",isOn:Binding(get:{studio.startAtLogin},set:studio.configureStartAtLogin)).font(.caption)
                if studio.startupNeedsApproval {Button("Allow background startup"){SMAppService.openSystemSettingsLoginItems()}.font(.caption)}
                Button("Export backup",action:studio.exportBackup).font(.caption)
                Button("Restore backup",action:studio.restoreBackup).font(.caption)
                Button("Permissions & Setup"){studio.refreshLoginStatus();studio.showPermissions=true}.font(.caption)
                Button("Show saved profiles") {NSWorkspace.shared.activateFileViewerSelecting([studio.storage])}.font(.caption)
            }.padding(24).frame(width:235).background(Color.black.opacity(0.18))
            VStack(alignment:.leading,spacing:20) {
                HStack(alignment:.top) {
                    VStack(alignment:.leading,spacing:6){Text(studio.editing?.name ?? "Choose an app").font(.largeTitle.bold());Text("Saved automatically · \(studio.editing?.mappings.values.filter{$0.action != "none"}.count ?? 0) assignments in this profile").foregroundColor(.secondary)}
                    Spacer()
                    Menu("Copy settings from…") {
                        ForEach(studio.profiles.filter{$0.id != studio.selectedProfile}) {profile in
                            Button(profile.name){studio.copySource=profile.id;studio.confirmCopy=true}
                        }
                    }.disabled(studio.editing==nil)
                    Toggle("Enable actions",isOn:$studio.actionsEnabled).toggleStyle(.switch).frame(width:165)
                }
                HStack(alignment:.center,spacing:22) {
                    if let image=studio.screenPreview {Image(nsImage:image).resizable().interpolation(.high).frame(width:114,height:114).cornerRadius(14).overlay(RoundedRectangle(cornerRadius:14).stroke(Color.white.opacity(0.15)))}
                    VStack(alignment:.leading,spacing:8){Text("CORE SCREEN").font(.caption.bold()).foregroundColor(.secondary);Text(studio.activeProfile?.name ?? studio.activeName).font(.title2.bold());Text("Follows the app you’re using. Opening this editor keeps the last app on screen.").font(.callout).foregroundColor(.secondary);if studio.activeProfile == nil {Button("Create profile for \(studio.activeName)",action:studio.addActive)}}
                    Spacer()
                    VStack(alignment:.leading){Text("Screen style for this profile").font(.caption).foregroundColor(.secondary);Picker("Screen",selection:Binding(get:{studio.editing?.display ?? "both"},set:studio.updateDisplay)){Text("Icon + name").tag("both");Text("Name").tag("name");Text("Icon").tag("icon")}.labelsHidden().frame(width:155)
                        Menu("Customize Program Icon") {
                            Button("Choose image…",action:studio.chooseCustomIcon)
                            Button("Use original program icon"){studio.setCustomIcon(nil)}.disabled(studio.editing?.customIconPNG == nil)
                        }.disabled(studio.editing==nil)
                        Menu("Rotate Console · \(studio.consoleOrientation)°") {
                            ForEach([0,90,180,270],id:\.self) {degrees in Button("\(degrees)°"){studio.rotateConsole(to:degrees)}}
                        }
                    }
                }.padding(18).background(Color.white.opacity(0.045)).cornerRadius(16)
                HStack {Image(systemName:studio.ledTestHex != nil ? "lightbulb.max":(studio.previewProfileID == nil ? "lightbulb":"eye"));Text("LED colors · \(studio.colorStatus)").font(.callout);Spacer()
                    Menu("Test lights") {
                        Button("All off"){studio.testLights("000000",name:"Off")}
                        Button("All red"){studio.testLights("FF0000",name:"Red")}
                        Button("All green"){studio.testLights("00FF00",name:"Green")}
                        Button("All blue"){studio.testLights("0000FF",name:"Blue")}
                        Button("All dim white"){studio.testLights("404040",name:"Dim white")}
                        Button("All white"){studio.testLights("FFFFFF",name:"White")}
                    }.disabled(!studio.connected)
                    if studio.ledTestHex != nil {Button("End light test",action:studio.endColorPreview)}
                    else if studio.previewProfileID != nil {Button("Follow active app",action:studio.endColorPreview)}
                }.padding(12).background(studio.previewProfileID == nil ? Color.white.opacity(0.035):Color.orange.opacity(0.12)).cornerRadius(10)
                if !studio.access { HStack{Image(systemName:"hand.raised");Text("Allow Accessibility access to send your assigned shortcuts and scroll actions.").font(.callout);Spacer();Button("Allow access",action:studio.requestAccess)}.padding(12).background(Color.orange.opacity(0.12)).cornerRadius(10) }
                HStack {Text("CONTROLS").font(.caption.bold()).foregroundColor(.secondary);Spacer();Button(studio.learning ? "Cancel selection":"Touch a control to select") {studio.learning.toggle()}.disabled(!studio.connected)}
                HStack(alignment:.top,spacing:18) {
                    ScrollView {VStack(alignment:.leading,spacing:15){if studio.modules.isEmpty {Text("Connect your controller to see its units. Profiles remain available while disconnected.").foregroundColor(.secondary).padding()};ForEach(studio.modules){m in
                        VStack(alignment:.leading,spacing:5){HStack {TextField("Name this unit",text:Binding(get:{studio.unitTitle(m)},set:{studio.renameUnit(m.id,$0)})).font(.headline).textFieldStyle(.plain);Button("Identify"){studio.identify(m)}.font(.caption);Menu {Button("Reset Module…"){studio.resetModuleUID=m.id;studio.confirmResetModule=true}} label:{Image(systemName:"ellipsis.circle")}.menuStyle(.borderlessButton).frame(width:22)};DisclosureGroup("Unit details") {Text("Hardware ID: \(m.id)").font(.caption.monospaced()).foregroundColor(.secondary)};if m.kind==10 {Toggle("Swap left/right labels",isOn:Binding(get:{studio.sliderFlipped[m.id,default:true]},set:{studio.swapSliderLabels(m.id,$0)})).font(.caption);Text("Labels match your mounted unit. Swap them if you rotate it.").font(.caption).foregroundColor(.secondary)};ForEach(studio.controls(for:m),id:\.0){event,title in
                            let c=ControlChoice(uid:m.id,event:event,title:studio.controlTitle(uid:m.id,event:event,fallback:title))
                            Button {studio.selectedControl=c;studio.learning=false} label:{HStack{Text(studio.controlTitle(uid:m.id,event:event,fallback:title));Spacer();if let mapping=studio.editing?.mappings[c.key],mapping.action != "none" {Image(systemName:"checkmark.circle.fill").foregroundColor(.mint)}}.padding(7).background(studio.selectedControl?.key==c.key ? Color.mint.opacity(0.16):Color.clear).cornerRadius(6)}.buttonStyle(.plain)
                        }}.padding(12).frame(maxWidth:.infinity,alignment:.leading).background(Color.white.opacity(0.035)).cornerRadius(10)
                    }}}.frame(width:275)
                    VStack(alignment:.leading,spacing:15) {
                        if let control=studio.selectedControl {
                            Text(studio.controlTitle(uid:control.uid,event:control.event,fallback:control.title)).font(.title3.bold())
                            TextField("Action label, e.g. Zoom in",text:field({$0.label},{$0.label=$1})).textFieldStyle(.roundedBorder)
                            if control.event.hasPrefix("pressed_") || control.event.hasPrefix("rotation") {
                                Menu("Choose a preset") {
                                    ForEach(ShortcutPreset.groups,id:\.self) { group in
                                        Menu(group) {
                                            ForEach(ShortcutPreset.all.filter{$0.group==group}) { preset in
                                                Button(preset.label+"  "+preset.shortcut){studio.applyPreset(preset)}
                                            }
                                        }
                                    }
                                }
                                Text("Applies one shortcut to this control. Presets are labeled by their source app; check that your app uses the same keys.").font(.caption).foregroundColor(.secondary)
                            }
                            Picker("Action",selection:field({$0.action},{$0.action=$1})){Text("Unassigned").tag("none");Text("Keyboard shortcut").tag("shortcut");Text("Scroll").tag("scroll");if control.event.hasPrefix("joystick") {Text("Page up / down").tag("pages")};if control.event.hasPrefix("set_"){Text("Mac output volume").tag("volume")}}
                            if studio.modules.first(where:{$0.id==control.uid})?.kind==24 {Button(studio.calibratingPad ? "Push pad up once…":"Set pad up/down by page",action:studio.calibratePad);Text("One page per push. Return to center before the next page; holding does not repeat.").font(.caption).foregroundColor(.secondary)}
                            if control.event=="rotation_left" || control.event=="rotation_right" {Button("Set ring to up/down scrolling",action:studio.ringScrolling);Text("Pairs the ring’s two directions: clockwise scrolls down; counterclockwise scrolls up.").font(.caption).foregroundColor(.secondary)}
                            if studio.mapping.action=="shortcut" { HStack{Text(studio.mapping.shortcut.isEmpty ? "No shortcut recorded":studio.mapping.shortcut).font(.title2.monospaced());Spacer();Button(studio.recordingShortcut ? "Press shortcut… (Esc cancels)":"Record shortcut",action:studio.captureShortcut)};Text("Click Record, then press the keys together. Shortcuts are sent only while this profile’s app is in front.").font(.caption).foregroundColor(.secondary) }
                            if control.event.hasPrefix("rotation") || control.event.hasPrefix("set") || control.event.hasPrefix("joystick") {Text("Sensitivity");Slider(value:field({$0.sensitivity},{$0.sensitivity=$1}),in:0.25...4);if studio.mapping.action != "shortcut" {Toggle(control.event=="rotation_left" || control.event=="rotation_right" ? "Reverse ring direction":"Reverse direction",isOn:Binding(get:{studio.mapping.invert},set:studio.setReverse))
                                if control.event.hasPrefix("joystick"),studio.mapping.action=="pages" {Text("To swap Page Up and Page Down, reverse only this assigned pad axis. Leave the other pad axis unchanged.").font(.caption).foregroundColor(.secondary)}
                                if (control.event=="rotation_left" || control.event=="rotation_right"),studio.mapping.action=="scroll" {Text("When both ring directions use Scroll, reversing either one updates both together.").font(.caption).foregroundColor(.secondary)}
                            };Text(control.event.hasPrefix("joystick") ? "Higher sensitivity needs a lighter page push. Release the pad between pushes; there is no timed delay.":"Endless encoders report changes, without physical endpoints. Assign clockwise and counterclockwise separately.").font(.caption).foregroundColor(.secondary)}
                            if studio.mapping.action=="shortcut",control.event.hasPrefix("set") {Text("For sliders, the shortcut repeats as the slider moves. Use volume for an absolute 0–100% control.").font(.caption).foregroundColor(.secondary)}
                            Divider()
                            ColorPicker("Saved LED color",selection:Binding(get:{studio.ledColor},set:studio.setLEDColor),supportsOpacity:false)
                            Text("Saved for \(studio.editing?.name ?? "this app") · \(studio.controlTitle(uid:control.uid,event:control.event,fallback:control.title))").font(.caption).foregroundColor(.secondary)
                            Button("Preview these colors",action:studio.previewColors).disabled(!studio.connected)
                            Text("Preview ends when you switch apps. Each app keeps its own saved colors.").font(.caption).foregroundColor(.secondary)
                            Text("Saved per app. The knob’s press and turn share its physical LED. Core and Orbiter use one module color. Black turns the light off.").font(.caption).foregroundColor(.secondary)
                            DisclosureGroup("White balance for this light") {
                                VStack(alignment:.leading,spacing:8) {
                                    Text("Optional correction shared across apps. This keeps your saved profile colors unchanged. The cause of the tint is still unconfirmed.").font(.caption).foregroundColor(.secondary)
                                    Button("Preview white on all lights"){studio.testLights("FFFFFF",name:"White")}.disabled(!studio.connected)
                                    ForEach(0..<3,id:\.self) { channel in
                                        HStack {
                                            Text(["Red","Green","Blue"][channel]).frame(width:45,alignment:.leading)
                                            Slider(value:Binding(get:{studio.whiteBalanceGain(channel)},set:{studio.setWhiteBalanceGain(channel,$0)}),in:0...1)
                                            Text("\(Int((studio.whiteBalanceGain(channel)*100).rounded()))%").monospacedDigit().frame(width:42)
                                        }
                                    }
                                    Text("Greenish: lower Green. Bluish: lower Blue. Adjust while viewing the physical light. Core buttons share one correction.").font(.caption).foregroundColor(.secondary)
                                    Button("Reset this light’s white balance",action:studio.resetWhiteBalance)
                                }.padding(.top,5)
                            }
                            Button("Clear assignment"){studio.update{$0=Mapping()}}
                        } else { Image(systemName:"slider.horizontal.3").font(.system(size:35)).foregroundColor(.mint);Text(studio.learning ? "Move or press the control you want to assign.":"Choose a control to begin.").font(.title3);Text("Each unit is remembered by its own identity, so rearranging modules won’t swap their assignments.").foregroundColor(.secondary) }
                        Spacer()
                        if studio.editing?.bundleID=="com.apple.finder" {Button("Add Finder starter assignments",action:studio.finderStarter).disabled(!studio.modules.contains(where:{$0.kind==8}));Text("Adds New window, Quick Look and New folder to one button unit. Existing assignments stay intact.").font(.caption).foregroundColor(.secondary)}
                    }.padding(20).frame(maxWidth:.infinity,maxHeight:.infinity,alignment:.topLeading).background(Color.white.opacity(0.035)).cornerRadius(12)
                }
                HStack{Circle().fill(studio.learning ? Color.orange:Color.mint).frame(width:6,height:6);Text(studio.connected ? studio.lastInput:"Connect controller first, then move a control to see activity.").font(.caption.monospaced()).lineLimit(1);Spacer();Button("Connection log"){studio.showLog=true}}.foregroundColor(.secondary)
                if !studio.message.isEmpty {HStack{Text(studio.message).font(.callout);Spacer();Button("Dismiss"){studio.message=""}}.padding(12).background(Color.orange.opacity(0.12)).cornerRadius(9)}
            }.padding(26).frame(minWidth:740)
        }.frame(minWidth:1030,minHeight:740).preferredColorScheme(.dark)
        .confirmationDialog("Replace settings for \(studio.editing?.name ?? "this profile")?",isPresented:$studio.confirmCopy,titleVisibility:.visible) {
            Button("Copy settings") {if let source=studio.copySource {studio.copySettings(from:source)}}
            Button("Cancel",role:.cancel){}
        } message: {Text("Assignments, LED colors and screen style will be copied. A backup of the current settings is kept automatically.")}
        .confirmationDialog("Reset module in \(studio.editing?.name ?? "this profile")?",isPresented:$studio.confirmResetModule,titleVisibility:.visible) {
            Button("Clear assignments and colors",role:.destructive){if let uid=studio.resetModuleUID {studio.resetModule(uid)}}
            Button("Cancel",role:.cancel){}
        } message:{Text("Clears only this module in the selected profile. Other modules, profiles, unit names and white balance are kept. A backup is saved first.")}
        .sheet(isPresented:$studio.showPermissions) {
            VStack(alignment:.leading,spacing:18) {
                HStack {Text("Permissions & Setup").font(.title2.bold());Spacer();Button("Done"){studio.showPermissions=false}}
                GroupBox {
                    VStack(alignment:.leading,spacing:8) {
                        Text("Accessibility · " + (studio.access ? "Allowed":"Needs permission")).font(.headline)
                        Text("Allows MONOcc to send your assigned keyboard shortcuts, scrolling and page commands to the app you are using.")
                        Text("System Settings → Privacy & Security → Accessibility → enable MONOcc.").font(.callout).foregroundColor(.secondary)
                        HStack {
                            Button("Open Accessibility Settings") {NSWorkspace.shared.open(URL(string:"x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Accessibility")!)}
                            if !studio.access {Button("Request access",action:studio.requestAccess)}
                        }
                        Text("If controls stop working after an update, check that the installed MONOcc in Applications is enabled here.").font(.caption).foregroundColor(.secondary)
                    }.frame(maxWidth:.infinity,alignment:.leading).padding(8)
                }
                GroupBox {
                    VStack(alignment:.leading,spacing:8) {
                        Text("Login startup · " + (studio.startupNeedsApproval ? "Needs approval":(studio.startAtLogin ? "Enabled":"Off"))).font(.headline)
                        Text("Starts MONOcc in the background when you log in, without opening setup.")
                        Text("System Settings → General → Login Items. Allow MONOcc if macOS asks for background startup approval.").font(.callout).foregroundColor(.secondary)
                        HStack {
                            Toggle("Start at login",isOn:Binding(get:{studio.startAtLogin},set:studio.configureStartAtLogin))
                            Spacer();Button("Open Login Items Settings"){SMAppService.openSystemSettingsLoginItems()}
                        }
                    }.frame(maxWidth:.infinity,alignment:.leading).padding(8)
                }
                Text("Close the setup window to keep the controller running. Reopen MONOcc from Applications or its menu bar icon for setup. Stop MONOcc and Quit ends the background controller.").font(.callout)
            }.padding(24).frame(width:640)
        }
        .sheet(isPresented:$studio.showLog) {VStack(alignment:.leading,spacing:14){HStack{Text("Connection log").font(.title2.bold());Spacer();Button("Done"){studio.showLog=false}};Text(studio.connection).foregroundColor(.secondary);ScrollView{Text(studio.logText).font(.system(.caption,design:.monospaced)).textSelection(.enabled).frame(maxWidth:.infinity,alignment:.leading)}}.padding(24).frame(width:700,height:430)}
    }
}

// The controller belongs to the background process; the setup window is optional.
final class EngineLease {
    private var descriptor:Int32 = -1
    init?(url:URL) {
        descriptor=open(url.path,O_CREAT | O_RDWR,S_IRUSR | S_IWUSR)
        guard descriptor >= 0 else{return nil}
        guard flock(descriptor,LOCK_EX | LOCK_NB)==0 else{close(descriptor);descriptor = -1;return nil}
    }
    deinit {if descriptor >= 0 {flock(descriptor,LOCK_UN);close(descriptor)}}
}

final class BackgroundAppDelegate:NSObject,NSApplicationDelegate,NSWindowDelegate,NSMenuDelegate {
    var studio:Studio?
    var setupWindow:NSWindow?
    var statusItem:NSStatusItem?
    private var lease:EngineLease?
    private var openObserver:NSObjectProtocol?
    private var menuStatus:NSMenuItem?
    private var loginItem:NSMenuItem?
    var backgroundLaunch:Bool {ProcessInfo.processInfo.arguments.contains("--background")}
    func applicationDidFinishLaunching(_ notification:Notification) {
        let root=ProcessInfo.processInfo.environment["MONOGRAM_PROFILE_HOME"].map{URL(fileURLWithPath:$0,isDirectory:true)} ?? FileManager.default.urls(for:.applicationSupportDirectory,in:.userDomainMask)[0].appendingPathComponent("MONOcc",isDirectory:true)
        try? FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
        guard let lease=EngineLease(url:root.appendingPathComponent("controller.lock")) else {
            if !backgroundLaunch {DistributedNotificationCenter.default().postNotificationName(Notification.Name("org.monocc.openSetup"),object:nil,userInfo:nil,deliverImmediately:true)}
            NSApp.terminate(nil);return
        }
        self.lease=lease
        let studio=Studio();self.studio=studio
        installMenu()
        openObserver=DistributedNotificationCenter.default().addObserver(forName:Notification.Name("org.monocc.openSetup"),object:nil,queue:.main){[weak self] _ in self?.showSetup()}
        studio.prepareBackgroundStartup();studio.connect()
        if !backgroundLaunch {showSetup()}
    }
    func installMenu() {
        let main=NSMenu();let appMenu=NSMenu();let appItem=NSMenuItem();appItem.submenu=appMenu;main.addItem(appItem)
        appMenu.addItem(withTitle:"Open Setup",action:#selector(showSetup),keyEquivalent:",").target=self
        appMenu.addItem(withTitle:"Close Setup",action:#selector(closeSetup),keyEquivalent:"w").target=self
        appMenu.addItem(.separator())
        let stop=appMenu.addItem(withTitle:"Stop MONOcc and Quit",action:#selector(stopService),keyEquivalent:"q");stop.target=self
        let edit=NSMenu();let editItem=NSMenuItem(title:"Edit",action:nil,keyEquivalent:"");editItem.submenu=edit;main.addItem(editItem)
        for (title,selector,key) in [("Undo",Selector(("undo:")),"z"),("Cut",#selector(NSText.cut(_:)),"x"),("Copy",#selector(NSText.copy(_:)),"c"),("Paste",#selector(NSText.paste(_:)),"v"),("Select All",#selector(NSText.selectAll(_:)),"a")] {edit.addItem(withTitle:title,action:selector,keyEquivalent:key)}
        NSApp.mainMenu=main
        let item=NSStatusBar.system.statusItem(withLength:NSStatusItem.squareLength)
        item.button?.image=NSImage(systemSymbolName:"slider.horizontal.3",accessibilityDescription:"MONOcc")
        item.button?.toolTip="MONOcc — background controller"
        let menu=NSMenu();menu.delegate=self
        menuStatus=menu.addItem(withTitle:"MONOcc running",action:nil,keyEquivalent:"")
        menu.addItem(withTitle:"Open Setup…",action:#selector(showSetup),keyEquivalent:"").target=self
        loginItem=menu.addItem(withTitle:"Start at login",action:#selector(toggleLogin),keyEquivalent:"");loginItem?.target=self
        menu.addItem(.separator());menu.addItem(withTitle:"Stop MONOcc and Quit",action:#selector(stopService),keyEquivalent:"").target=self
        item.menu=menu;statusItem=item
    }
    func menuNeedsUpdate(_ menu:NSMenu) {
        studio?.refreshLoginStatus()
        menuStatus?.title="MONOcc · \(studio?.connection ?? "Starting…")"
        loginItem?.state=studio?.startAtLogin == true ? .on:.off
        loginItem?.title=studio?.startupNeedsApproval == true ? "Allow startup in System Settings…":"Start at login"
    }
    @objc func toggleLogin() {
        guard let studio=studio else{return}
        if studio.startupNeedsApproval {SMAppService.openSystemSettingsLoginItems()} else {studio.configureStartAtLogin(!studio.startAtLogin)}
    }
    @objc func showSetup() {
        guard let studio=studio else{return}
        NSApp.setActivationPolicy(.regular)
        if setupWindow == nil {
            let window=NSWindow(contentRect:NSRect(x:0,y:0,width:1100,height:800),styleMask:[.titled,.closable,.miniaturizable,.resizable],backing:.buffered,defer:false)
            window.title="MONOcc Setup";window.isReleasedWhenClosed=false;window.delegate=self
            window.contentView=NSHostingView(rootView:MainView(studio:studio))
            window.center();setupWindow=window
        }
        setupWindow?.makeKeyAndOrderFront(nil);NSApp.activate(ignoringOtherApps:true)
    }
    @objc func closeSetup() {setupWindow?.close()}
    func windowWillClose(_ notification:Notification) {
        studio?.finishSetup()
        NSApp.setActivationPolicy(.accessory)
        studio?.appendLog("Setup closed; background controller remains active.")
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender:NSApplication)->Bool {false}
    func applicationShouldHandleReopen(_ sender:NSApplication,hasVisibleWindows flag:Bool)->Bool {showSetup();return true}
    @objc func stopService() {studio?.disconnect();NSApp.terminate(nil)}
    func applicationShouldTerminate(_ sender:NSApplication)->NSApplication.TerminateReply {
        // Quit explicitly stops the controller. Closing the setup window never does.
        studio?.disconnect();return .terminateNow
    }
}

@main struct MonogramProfilesApp {
    static func main() {
        let app=NSApplication.shared
        app.setActivationPolicy(.accessory)
        let delegate=BackgroundAppDelegate();app.delegate=delegate
        withExtendedLifetime(delegate){app.run()}
    }
}
