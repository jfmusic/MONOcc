@main struct Verification {
    static func main() {
        _ = NSApplication.shared
        let studio=Studio()
        assert(!studio.actionsEnabled)
        studio.actionsEnabled=true
        let preferenceData=try! Data(contentsOf:studio.storage.deletingLastPathComponent().appendingPathComponent("preferences.json"))
        assert(try! JSONDecoder().decode([String:Bool].self,from:preferenceData)["actionsEnabled"]==true)
        studio.actionsEnabled=false
        assert(studio.profiles.contains(where:{$0.bundleID=="com.apple.finder"}))
        let dp=studio.profiles.filter{$0.bundleID=="com.motu.DigitalPerformer"}
        assert(dp.contains{$0.appPath.contains("11.36")})
        assert(dp.contains{$0.appPath.contains("12.0.0")})
        assert(Set(dp.map{$0.id}).count==dp.count)
        studio.activeBundle="com.motu.DigitalPerformer";studio.activePath="/Applications/Digital Performer 12.0.0.app"
        assert(studio.activeProfile?.appPath==studio.activePath)
        studio.renameUnit("?wI-0","Top knobs")
        assert(studio.unitTitle(Module(id:"?wI-0",index:99,kind:9,name:"Tri-Dial"))=="Top knobs")
        studio.receive("{\"modules\":[{\"index\":6,\"uid\":\"?wI-0\",\"kind\":9,\"name\":\"Tri-Dial\"},{\"index\":2,\"uid\":\"?x-|q\",\"kind\":24,\"name\":\"Orbiter\"}]}")
        assert(studio.modules.count==2)
        studio.learning=true
        studio.receive("[in] i=6 Tri-Dial raw=[0,0,1,0,0,0,0,0] changed=v2 signed8=[0,0,1,0,0,0,0,0] => rotation_right_2=1 set_2=4")
        assert(studio.selectedControl?.uid=="?wI-0")
        assert(studio.selectedControl?.event=="rotation_right_2")
        studio.update{$0.label="Test zoom";$0.action="shortcut";$0.keyCode=24;$0.shortcut="⌘+"}
        let beforePreset=studio.profiles
        let originalColors=studio.editing!.ledColors
        let savePreset=ShortcutPreset.all.first{$0.group=="Common shortcuts" && $0.label=="Save"}!
        assert(savePreset.keyCode==1 && savePreset.modifiers==UInt64(NSEvent.ModifierFlags.command.rawValue))
        assert(ShortcutPreset.all.count==307)
        assert(ShortcutPreset.all.contains{$0.group=="Safari" && $0.label=="New Tab" && $0.keyCode==17})
        assert(ShortcutPreset.all.contains{$0.group=="Chrome" && $0.keyCode==121})
        studio.applyPreset(savePreset)
        assert(studio.mapping==savePreset.mapping)
        assert(studio.editing!.ledColors==originalColors)
        for profile in beforePreset where profile.id != studio.selectedProfile {assert(studio.profiles.first{$0.id==profile.id}==profile)}
        for (key,value) in beforePreset.first(where:{$0.id==studio.selectedProfile})!.mappings where key != studio.selectedControl!.key {assert(studio.editing!.mappings[key]==value)}
        let savedPresetProfiles=try! JSONDecoder().decode([AppProfile].self,from:Data(contentsOf:studio.storage))
        assert(savedPresetProfiles==studio.profiles)
        studio.profiles=beforePreset;studio.save()
        let original=studio.mapping
        studio.selectedControl=ControlChoice(uid:"?wI-0",event:"pressed_2",title:"Knob 2 press")
        assert(studio.mapping.action=="none")
        studio.setLEDColor(Color(red:1,green:0,blue:0))
        studio.selectedControl=ControlChoice(uid:"?wI-0",event:"rotation_right_2",title:"Knob 2")
        assert(studio.mapping==original)
        assert(studio.editing?.ledColors["?wI-0|1"]=="FF0000")
        studio.selectedControl=ControlChoice(uid:"?x-|q",event:"rotation_right",title:"Ring")
        assert(studio.ledKey=="?x-|q|0")
        studio.setLEDColor(Color(red:0,green:0,blue:1))
        let data=try! Data(contentsOf:studio.storage)
        let restored=try! JSONDecoder().decode([AppProfile].self,from:data)
        assert(restored==studio.profiles)
        // A re-enumeration changes index but preserves the saved identity.
        studio.receive("{\"modules\":[{\"index\":99,\"uid\":\"?wI-0\",\"kind\":9,\"name\":\"Tri-Dial\"}]}")
        studio.learning=true
        studio.receive("[in] i=99 Tri-Dial raw=[0,0,1,0,0,0,0,0] changed=v2 signed8=[0,0,1,0,0,0,0,0] => rotation_right_2=1 set_2=8")
        assert(studio.mapping==original)
        studio.receive("{\"modules\":[{\"index\":2,\"uid\":\"?x-|q\",\"kind\":24,\"name\":\"Orbiter\"}]}")
        studio.selectedControl=ControlChoice(uid:"?x-|q",event:"rotation_right",title:"Ring")
        studio.ringScrolling()
        assert(studio.editing?.mappings["?x-|q|rotation_left"]?.invert==true)
        assert(studio.editing?.mappings["?x-|q|rotation_right"]?.invert==true)
        studio.setReverse(false)
        assert(studio.editing?.mappings["?x-|q|rotation_left"]?.invert==false)
        assert(studio.editing?.mappings["?x-|q|rotation_right"]?.invert==false)
        studio.calibratePad()
        studio.receive("[in] i=2 Orbiter raw=[0,0,0,0,150,0,0,0] changed=v4 signed8=[0,0,0,0,-106,0,0,0] => joystick_y=150")
        assert(studio.mapping.action=="pages")
        assert(studio.mapping.invert)
        let pageKey="?x-|q|pad"
        assert(studio.pageTrigger(key:pageKey,value:40,reverse:true)==nil)
        studio.receive("[in] i=2 Orbiter raw=[0,0,0,0,0,0,0,0] changed=v4 signed8=[0,0,0,0,0,0,0,0] => (neutral)")
        assert(studio.pageTrigger(key:pageKey,value:40,reverse:true)==true)
        assert(studio.pageTrigger(key:pageKey,value:40,reverse:true)==nil)
        studio.receive("[in] i=2 Orbiter raw=[0,0,0,0,0,0,0,0] changed=v4 signed8=[0,0,0,0,0,0,0,0] => (neutral)")
        assert(studio.pageTrigger(key:pageKey,value:-150,reverse:true)==false)
        studio.activeBundle=studio.editing!.bundleID;studio.activePath=studio.editing!.appPath
        // Off-center return and perpendicular drift must not require a heavy second push.
        studio.releasePagePad(uid:"?x-|q",x:70,y:20) // calibrated page axis is Y
        assert(studio.pageTrigger(key:pageKey,value:40,reverse:true)==true)
        studio.releasePagePad(uid:"?x-|q",x:0,y:28) // still held, above release threshold
        assert(studio.pageTrigger(key:pageKey,value:-40,reverse:true)==nil)
        studio.releasePagePad(uid:"?x-|q",x:60,y:20)
        assert(studio.pageTrigger(key:pageKey,value:-40,reverse:true)==false)
        assert(studio.pageTrigger(key:pageKey,value:-80,reverse:true)==nil)
        studio.releasePagePad(uid:"?x-|q",x:0,y:0)
        assert(studio.pageTrigger(key:pageKey,value:25,reverse:false,sensitivity:1)==nil)
        assert(studio.pageTrigger(key:pageKey,value:25,reverse:false,sensitivity:2)==false)
        studio.releasePagePad(uid:"?x-|q",x:0,y:0)
        assert(studio.pageTrigger(key:pageKey,value:40,reverse:false,sensitivity:0.5)==nil)
        assert(studio.pageTrigger(key:pageKey,value:61,reverse:false,sensitivity:0.5)==false)
        assert(studio.pageThreshold(sensitivity:Double.nan)==30)
        print("PASS: forgiving calibrated-axis release, no hold repeat, immediate opposite push and page sensitivity")
        let dpSource=studio.profiles.first{$0.appPath.contains("11.36")}!
        let dpTarget=studio.profiles.first{$0.appPath.contains("12.0.0")}!
        studio.selectedProfile=dpSource.id
        studio.selectedControl=ControlChoice(uid:"?wI-0",event:"pressed_1",title:"Test")
        studio.update{$0.label="Transport";$0.action="shortcut";$0.keyCode=49;$0.shortcut="Space"}
        studio.selectedProfile=dpTarget.id
        studio.copySettings(from:dpSource.id)
        assert(studio.editing?.id==dpTarget.id && studio.editing?.appPath==dpTarget.appPath)
        assert(studio.editing?.mappings==studio.profiles.first{$0.id==dpSource.id}!.mappings)
        studio.update{$0.label="DP12 only"}
        assert(studio.profiles.first{$0.id==dpSource.id}!.mappings["?wI-0|pressed_1"]?.label=="Transport")
        // Page assignments are exclusive and a stale conflicting axis is repaired.
        studio.selectedControl=ControlChoice(uid:"?x-|q",event:"joystick_x",title:"Pad")
        studio.update{$0.action="pages";$0.label="Pad up / down · one page"}
        studio.selectedControl=ControlChoice(uid:"?x-|q",event:"joystick_y",title:"Pad")
        studio.update{$0.action="pages"}
        assert(studio.editing?.mappings["?x-|q|joystick_x"]==nil)
        let selected=studio.profiles.firstIndex{$0.id==studio.selectedProfile}!
        studio.profiles[selected].mappings["?x-|q|joystick_x"]=Mapping(label:"Pad up / down · one page",action:"pages")
        studio.repairPadBindings()
        assert(studio.editing?.mappings["?x-|q|joystick_y"]==nil)
        assert(studio.editing?.mappings["?x-|q|joystick_x"]?.action=="pages")
        // Every default light gets a real white command, not a missing setting.
        studio.modules=[25,8,24,10,10,8,10,9,8].enumerated().map{Module(id:"test-unit-\($0.offset)",index:$0.offset,kind:$0.element,name:"Test")}
        let lights=studio.ledPlan()
        assert(lights.count==23)
        assert(lights.allSatisfy{($0["rgb"] as? [Int])==[255,255,255]})
        let labelUnit=studio.modules.first{$0.kind==10}!
        assert(studio.controls(for:labelUnit).map{$0.0}==["set_3","set_2","set_1"])
        assert(studio.controlTitle(uid:labelUnit.id,event:"set_1",fallback:"")=="Right slider")
        assert(studio.controlTitle(uid:labelUnit.id,event:"set_3",fallback:"")=="Left slider")
        let unchangedProfiles=studio.profiles
        studio.selectedControl=ControlChoice(uid:labelUnit.id,event:"set_1",title:"Right slider")
        assert(studio.ledKey==labelUnit.id+"|0")
        studio.swapSliderLabels(labelUnit.id,false)
        assert(studio.controlTitle(uid:labelUnit.id,event:"set_1",fallback:"")=="Left slider")
        assert(studio.controls(for:labelUnit).map{$0.0}==["set_1","set_2","set_3"])
        studio.swapSliderLabels(labelUnit.id,true)
        studio.previewColors()
        assert(studio.previewProfileID==studio.selectedProfile)
        assert(studio.colorStatus.hasPrefix("Preview:"))
        studio.endColorPreview()
        assert(studio.previewProfileID==nil)
        assert(studio.profiles==unchangedProfiles)
        let backup=ProfileBackup(profiles:studio.profiles,unitNames:studio.unitNames,sliderFlipped:studio.sliderFlipped)
        let restoredBackup=try! JSONDecoder().decode(ProfileBackup.self,from:JSONEncoder().encode(backup))
        assert(restoredBackup.profiles==studio.profiles && restoredBackup.unitNames==studio.unitNames && restoredBackup.sliderFlipped==studio.sliderFlipped)
        for mode in ["name","icon","both"] {
            let (frame,image)=makeScreen(name:"Affinity Publisher",icon:appIcon("/Applications/Affinity Publisher 2.app"),mode:mode)
            assert(frame.count==115200)
            assert(Set(frame).count>10)
            try! frame.write(to:URL(fileURLWithPath:"work/screen-"+mode+".rgb565"))
            let png=NSBitmapImageRep(data:image.tiffRepresentation!)!.representation(using:.png,properties:[:])!
            try! png.write(to:URL(fileURLWithPath:"work/screen-"+mode+".png"))
        }
        // A diagnostic overrides every physical light without touching saved data.
        let beforeLightTest=studio.profiles
        studio.connected=true
        for (hex,rgb) in [("000000",[0,0,0]),("FF0000",[255,0,0]),("00FF00",[0,255,0]),("0000FF",[0,0,255]),("404040",[64,64,64]),("FFFFFF",[255,255,255])] {
            studio.testLights(hex,name:hex)
            let plan=studio.ledPlan()
            assert(!plan.isEmpty)
            assert(plan.allSatisfy{($0["rgb"] as? [Int])==rgb})
            assert(studio.profiles==beforeLightTest)
            studio.activeBundle="test.other.app";studio.activePath="/Applications/Other.app"
            assert(studio.ledPlan().allSatisfy{($0["rgb"] as? [Int])==rgb})
        }
        studio.endColorPreview()
        assert(studio.ledTestHex==nil && studio.profiles==beforeLightTest)
        assert(studio.ledPlan().allSatisfy{($0["rgb"] as? [Int])==[255,255,255]})
        let beforeBalanceProfiles=studio.profiles
        studio.receive("{\"modules\":[{\"index\":6,\"uid\":\"balance-test\",\"kind\":8,\"name\":\"Tri-Button\"}]}")
        studio.selectedControl=ControlChoice(uid:"balance-test",event:"pressed_1",title:"Button 1")
        studio.testLights("FFFFFF",name:"White")
        studio.setWhiteBalanceGain(1,0.5)
        let balanced=studio.ledPlan()
        assert(balanced.first{($0["sub"] as? Int)==0}?["rgb"] as? [Int]==[255,128,255])
        assert(balanced.first{($0["sub"] as? Int)==1}?["rgb"] as? [Int]==[255,255,255])
        assert(studio.profiles==beforeBalanceProfiles)
        let balanceData=try! Data(contentsOf:studio.storage.deletingLastPathComponent().appendingPathComponent("white-balance.json"))
        assert(try! JSONDecoder().decode([String:[Double]].self,from:balanceData)["balance-test|0"]==[1,0.5,1])
        assert(studio.validWhiteBalance(["bad":[1,2],"invalid":[1,-1,1],"good":[1,0.5,1]]).count==1)
        let balanceBackup=ProfileBackup(profiles:studio.profiles,unitNames:studio.unitNames,whiteBalance:studio.whiteBalance)
        assert(try! JSONDecoder().decode(ProfileBackup.self,from:JSONEncoder().encode(balanceBackup)).whiteBalance==studio.whiteBalance)
        studio.resetWhiteBalance()
        assert(studio.whiteBalance["balance-test|0"]==nil && studio.profiles==beforeBalanceProfiles)
        studio.endColorPreview()
        // Reset affects only the selected module in the selected profile, and is recoverable.
        let resetIndex=studio.profiles.firstIndex{$0.id==studio.selectedProfile}!
        studio.profiles[resetIndex].mappings["balance-test|pressed_1"]=Mapping(label:"Reset me",action:"shortcut")
        studio.profiles[resetIndex].mappings["balance-test-other|pressed_1"]=Mapping(label:"Keep me",action:"shortcut")
        studio.profiles[resetIndex].ledColors["balance-test|0"]="FF0000"
        studio.profiles[resetIndex].ledColors["balance-test-other|0"]="00FF00"
        studio.save()
        let beforeReset=studio.profiles
        studio.resetModule("balance-test")
        assert(studio.profiles[resetIndex].mappings["balance-test|pressed_1"]==nil)
        assert(studio.profiles[resetIndex].ledColors["balance-test|0"]==nil)
        assert(studio.profiles[resetIndex].mappings["balance-test-other|pressed_1"]==beforeReset[resetIndex].mappings["balance-test-other|pressed_1"])
        assert(studio.profiles[resetIndex].ledColors["balance-test-other|0"]=="00FF00")
        for i in studio.profiles.indices where i != resetIndex {assert(studio.profiles[i]==beforeReset[i])}
        let resetBackups=try! FileManager.default.contentsOfDirectory(at:studio.storage.deletingLastPathComponent(),includingPropertiesForKeys:nil).filter{$0.lastPathComponent.hasPrefix("Before module reset")}
        assert(resetBackups.count==1)
        assert(try! JSONDecoder().decode(ProfileBackup.self,from:Data(contentsOf:resetBackups[0])).profiles==beforeReset)
        // Images are embedded in profiles and backups, and affect rendered screen pixels.
        let icon=NSImage(size:NSSize(width:240,height:240));icon.lockFocus();NSColor.red.setFill();NSBezierPath(rect:NSRect(x:0,y:0,width:240,height:240)).fill();icon.unlockFocus()
        let iconData=NSBitmapImageRep(data:icon.tiffRepresentation!)!.representation(using:.png,properties:[:])!
        studio.activeBundle=studio.editing!.bundleID;studio.activePath=studio.editing!.appPath
        let oldFrame=makeScreen(name:studio.editing!.name,icon:profileIcon(studio.editing!),mode:"icon").0
        studio.setCustomIcon(iconData)
        assert(studio.editing?.customIconPNG==iconData)
        assert(makeScreen(name:studio.editing!.name,icon:profileIcon(studio.editing!),mode:"icon").0 != oldFrame)
        let roundtrip=try! JSONDecoder().decode([AppProfile].self,from:Data(contentsOf:studio.storage))
        assert(roundtrip[resetIndex].customIconPNG==iconData)
        let iconBackup=ProfileBackup(profiles:studio.profiles,unitNames:studio.unitNames,consoleOrientation:90)
        let restoredIconBackup=try! JSONDecoder().decode(ProfileBackup.self,from:JSONEncoder().encode(iconBackup))
        assert(restoredIconBackup.profiles[resetIndex].customIconPNG==iconData && restoredIconBackup.consoleOrientation==90)
        let savedIconProfiles=studio.profiles
        studio.setCustomIcon(Data("invalid".utf8));assert(studio.profiles==savedIconProfiles)
        studio.setCustomIcon(nil);assert(studio.editing?.customIconPNG==nil)
        for degrees in [90,180,270,0] {studio.rotateConsole(to:degrees);assert(studio.consoleOrientation==degrees)}
        studio.rotateConsole(to:45);assert(studio.consoleOrientation==0)
        assert(try! JSONDecoder().decode(Int.self,from:Data(contentsOf:studio.storage.deletingLastPathComponent().appendingPathComponent("console-orientation.json")))==0)
        let oldProfile=Data(#"{"id":"legacy","name":"Legacy","bundleID":"test.legacy","appPath":"/Legacy.app","display":"both","mappings":{},"ledColors":{}}"#.utf8)
        assert(try! JSONDecoder().decode(AppProfile.self,from:oldProfile).customIconPNG==nil)
        print("PASS: scoped module reset and backup, custom icon rendering/persistence, legacy profiles, console orientation")
        // Closing setup ends temporary editor modes without stopping the controller.
        let beforeClosing=studio.profiles
        let beforeActions=studio.actionsEnabled
        studio.learning=true;studio.recordingShortcut=true;studio.calibratingPad=true
        studio.testLights("FF0000",name:"Red")
        studio.finishSetup()
        assert(!studio.learning && !studio.recordingShortcut && !studio.calibratingPad)
        assert(studio.ledTestHex==nil && studio.previewProfileID==nil && studio.identifyingUID==nil)
        assert(studio.profiles==beforeClosing && studio.actionsEnabled==beforeActions && studio.connected)
        let delegate=BackgroundAppDelegate()
        assert(!delegate.applicationShouldTerminateAfterLastWindowClosed(NSApplication.shared))
        let lockURL=studio.storage.deletingLastPathComponent().appendingPathComponent("test-controller.lock")
        var firstLease=EngineLease(url:lockURL)
        assert(firstLease != nil && EngineLease(url:lockURL)==nil)
        firstLease=nil
        assert(EngineLease(url:lockURL) != nil)
        // Intentional disconnect must suppress retries; unexpected exits are retried after backoff.
        let retryFolder=studio.storage.deletingLastPathComponent().appendingPathComponent("retry-test")
        let retryStudio=Studio(profileHome:retryFolder)
        assert(!retryStudio.shouldReconnect(at:Date.distantFuture))
        retryStudio.connect() // Verification bundle deliberately has no bundled device helper.
        assert(!retryStudio.shouldReconnect(at:Date()))
        assert(retryStudio.shouldReconnect(at:Date.distantFuture))
        retryStudio.disconnect();assert(!retryStudio.shouldReconnect(at:Date.distantFuture))
        print("PASS: background window lifecycle, setup cleanup, single controller ownership and reconnect intent/backoff")
        let badFolder=studio.storage.deletingLastPathComponent().appendingPathComponent("unreadable-test")
        try! FileManager.default.createDirectory(at:badFolder,withIntermediateDirectories:true)
        let badFile=badFolder.appendingPathComponent("profiles.json")
        let unreadableOriginal=Data("broken json keep me".utf8);try! unreadableOriginal.write(to:badFile)
        let badStudio=Studio(profileHome:badFolder)
        badStudio.save()
        assert(try! Data(contentsOf:badFile)==unreadableOriginal)
        print("PASS: control learning, stable module identity, independent assignments, per-control colors, profile persistence, and all three screen modes")
    }
}
