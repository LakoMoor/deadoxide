import java.util.Arrays;
import zombie.ZomboidFileSystem;
import zombie.Lua.LuaManager;
import zombie.Lua.LuaEventManager;
import zombie.core.random.RandStandard;
import se.krka.kahlua.luaj.compiler.LuaCompiler;

public class LuaEventApiProbe {
    static void scenario(String name, String setup, int rounds) throws Exception {
        String event="Probe"+name; LuaEventManager.AddEvent(event);
        LuaManager.env.rawset("trace", "");
        String code=setup.replace("$E", "Events."+event);
        Object[] result=LuaManager.thread.pcall(LuaCompiler.loadstring(code,"event-probe",LuaManager.env),new Object[0]);
        if (!Boolean.TRUE.equals(result[0])) throw new IllegalStateException(Arrays.toString(result));
        for(int i=0;i<rounds;i++) {
            LuaEventManager.triggerEvent(event);
            System.out.println("EVENT "+name+"."+(i+1)+"="+LuaManager.env.rawget("trace"));
        }
    }
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-event-probe"); ZomboidFileSystem.instance.init();
        RandStandard.INSTANCE.init(); LuaManager.init();
        scenario("duplicate", "local function a() trace=trace..'A' end; local function b() trace=trace..'B' end; $E.Add(a); $E.Add(a); $E.Add(b)",1);
        scenario("remove", "local function a() trace=trace..'A' end; local function b() trace=trace..'B' end; $E.Add(a); $E.Add(b); $E.Remove(a)",1);
        scenario("remove_next", "local b; local function a() trace=trace..'A'; $E.Remove(b) end; b=function() trace=trace..'B' end; local function c() trace=trace..'C' end; $E.Add(a); $E.Add(b); $E.Add(c)",1);
        scenario("add_during", "local function c() trace=trace..'C' end; local function a() trace=trace..'A'; $E.Add(c) end; local function b() trace=trace..'B' end; $E.Add(a); $E.Add(b)",2);
        scenario("remove_self", "local a; a=function() trace=trace..'A'; $E.Remove(a) end; local function b() trace=trace..'B' end; $E.Add(a); $E.Add(b)",2);
        scenario("error", "local function a() trace=trace..'A'; error('probe error') end; local function b() trace=trace..'B' end; $E.Add(a); $E.Add(b)",1);
    }
}
