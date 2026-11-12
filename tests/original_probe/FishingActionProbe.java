import java.nio.file.*;
import java.util.*;
import java.lang.reflect.*;
import zombie.GameTime;
import zombie.Lua.LuaManager;
import zombie.ZomboidFileSystem;
import zombie.core.random.RandStandard;
import zombie.iso.FishSchoolManager;
import zombie.util.PZCalendar;
import gnu.trove.map.hash.TLongObjectHashMap;
import se.krka.kahlua.luaj.compiler.LuaCompiler;
public class FishingActionProbe {
  static void run(String source,String name) throws Exception {
    Object[] result=LuaManager.thread.pcall(LuaCompiler.loadstring(source,name,LuaManager.env),new Object[0]);
    if(!Boolean.TRUE.equals(result[0])) throw new IllegalStateException(Arrays.toString(result));
  }
  @SuppressWarnings("unchecked")
  public static void main(String[] args) throws Exception {
    ZomboidFileSystem.instance.setCacheDir("/tmp/pz-fishing-action-probe"); ZomboidFileSystem.instance.init(); RandStandard.INSTANCE.init(); LuaManager.init();
    GameTime time=new GameTime(); GameTime.setInstance(time); GregorianCalendar c=new GregorianCalendar(TimeZone.getTimeZone("UTC")); c.clear(); time.setCalender(new PZCalendar(c)); time.setYear(1993); time.setMonth(6); time.setDay(8); time.setTimeOfDay(10f);
    FishSchoolManager manager=FishSchoolManager.getInstance();
    Field f=FishSchoolManager.class.getDeclaredField("chumPoints"); f.setAccessible(true); TLongObjectHashMap<FishSchoolManager.ChumData> chum=(TLongObjectHashMap<FishSchoolManager.ChumData>)f.get(null); chum.clear();
    run("require=function() end; ISBaseTimedAction={derive=function() return {} end}; sendServerCommand=function(...) sent=true end; sendRemoveItemFromContainer=function(...) removed=true end", "stubs");
    run(Files.readString(Path.of(args[0])),"AddChumToWaterAction.lua");
    run("local item={getHungChange=function() return -0.10125 end}; local inv={Remove=function() end}; local obj={chum=item,square={getX=function() return 100 end,getY=function() return 100 end},character={getInventory=function() return inv end}}; success=AddChumToWaterAction.complete(obj)","action");
    Method h=FishSchoolManager.class.getDeclaredMethod("coordsToHash",int.class,int.class); h.setAccessible(true);
    FishSchoolManager.ChumData v=chum.get((Long)h.invoke(manager,100,100));
    System.out.printf("ACTION success=%s sent=%s removed=%s max=%d end=%d%n",LuaManager.env.rawget("success"),LuaManager.env.rawget("sent"),LuaManager.env.rawget("removed"),v.maxForceTime,v.endTime);
  }
}
