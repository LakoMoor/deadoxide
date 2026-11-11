import java.io.*;
import zombie.GameTime;
import zombie.core.random.RandStandard;
import zombie.Lua.LuaManager;
import zombie.ZomboidFileSystem;
import zombie.network.GameServer;
import zombie.network.ServerOptions;
public class ClockPauseProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-time-probe"); ZomboidFileSystem.instance.init();
        RandStandard.INSTANCE.init(); LuaManager.init();
        GameTime t = new GameTime(); GameTime.setInstance(t);
        try(var in=new DataInputStream(new BufferedInputStream(new FileInputStream(args[0])))){t.load(in);}
        var stamp=GameTime.class.getDeclaredMethod("setMinutesStamp"); stamp.setAccessible(true); stamp.invoke(t);
        var prev=GameTime.class.getDeclaredField("previousMinuteStamp"); prev.setAccessible(true); prev.setLong(t,t.getMinutesStamp());
        var mod=GameTime.class.getDeclaredField("minutesMod"); mod.setAccessible(true); mod.setInt(t,t.getMinutes()/10);
        ServerOptions.instance.pauseEmpty.setValue(true); GameServer.server=true;
        var sync=GameTime.class.getDeclaredField("lastClockSync"); sync.setAccessible(true); sync.setLong(t,System.currentTimeMillis());
        int before=Float.floatToRawIntBits(t.getTimeOfDay()); boolean paused=GameTime.isGamePaused();
        t.update(false);
        System.out.printf("PAUSE %s=%08x,%08x%n",paused,before,Float.floatToRawIntBits(t.getTimeOfDay()));
    }
}
