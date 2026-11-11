import java.io.*;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import zombie.GameTime;
import zombie.SandboxOptions;
import zombie.ZomboidFileSystem;
import zombie.Lua.LuaManager;
import zombie.Lua.LuaEventManager;
import zombie.core.PerformanceSettings;
import zombie.core.random.RandStandard;
import zombie.erosion.ErosionMain;
import zombie.iso.sprite.IsoSpriteManager;
import se.krka.kahlua.luaj.compiler.LuaCompiler;
import se.krka.kahlua.vm.JavaFunction;

public class ClockSavedEventsProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-time-probe"); ZomboidFileSystem.instance.init();
        RandStandard.INSTANCE.init(); LuaManager.init(); PerformanceSettings.setLockFPS(30);
        new ErosionMain(IsoSpriteManager.instance, false);
        var trace=new ArrayList<String>();
        LuaManager.env.rawset("recordClockEvent", (JavaFunction)(frame, n)->{trace.add((String)frame.get(0));return 0;});
        for(String name:new String[]{"EveryOneMinute","EveryTenMinutes","EveryHours","EveryDays"}) {
            var event=LuaEventManager.AddEvent(name);
            event.callbacks.add(LuaCompiler.loadstring("recordClockEvent('"+name+"')","clock-saved-events",LuaManager.env));
        }
        SandboxOptions.instance.load(ByteBuffer.wrap(Files.readAllBytes(Path.of(args[1]))));
        GameTime t=new GameTime(); GameTime.setInstance(t);
        try(var in=new DataInputStream(new BufferedInputStream(new FileInputStream(args[0])))){t.load(in);}
        t.setMinutesPerDay(SandboxOptions.instance.getDayLengthMinutes()); t.setMultiplier(1f); t.fpsMultiplier=2f;
        t.setLastTimeOfDay(t.getTimeOfDay()); t.lastLastTimeOfDay=t.getTimeOfDay();
        var stamp=GameTime.class.getDeclaredMethod("setMinutesStamp"); stamp.setAccessible(true); stamp.invoke(t);
        var prev=GameTime.class.getDeclaredField("previousMinuteStamp"); prev.setAccessible(true); prev.setLong(t,t.getMinutesStamp());
        var mod=GameTime.class.getDeclaredField("minutesMod"); mod.setAccessible(true); mod.setInt(t,t.getMinutes()/10);
        for(int tick=1;tick<=150;tick++) {
            trace.clear(); t.update(false);
            for(String name:trace) System.out.printf("EVENT tick=%d name=%s%n",tick,name);
        }
        System.out.printf("END bits=%08x nights=%d%n",Float.floatToRawIntBits(t.getTimeOfDay()),t.getNightsSurvived());
    }
}
