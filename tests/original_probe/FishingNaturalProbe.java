import java.nio.file.*;
import java.lang.reflect.*;
import java.util.*;
import zombie.ZomboidFileSystem;
import zombie.GameTime;
import zombie.SandboxOptions;
import zombie.Lua.LuaManager;
import zombie.core.random.RandStandard;
import zombie.iso.FishSchoolManager;
import zombie.iso.zones.Zone;
import zombie.util.PZCalendar;
import se.krka.kahlua.luaj.compiler.LuaCompiler;
import gnu.trove.map.hash.TLongObjectHashMap;
public class FishingNaturalProbe {
 static void run(String s)throws Exception{Object[] r=LuaManager.thread.pcall(LuaCompiler.loadstring(s,"zones",LuaManager.env),new Object[0]);if(!Boolean.TRUE.equals(r[0]))throw new IllegalStateException(Arrays.toString(r));}
 @SuppressWarnings("unchecked") public static void main(String[] a)throws Exception{
  ZomboidFileSystem.instance.setCacheDir("/tmp/pz-natural-probe");ZomboidFileSystem.instance.init();RandStandard.INSTANCE.init();LuaManager.init();run(Files.readString(Path.of(a[0])));
  GameTime game=new GameTime();GameTime.setInstance(game);GregorianCalendar cal=new GregorianCalendar(TimeZone.getTimeZone("UTC"));cal.clear();game.setCalender(new PZCalendar(cal));game.setYear(1993);game.setMonth(6);game.setDay(8);
  FishSchoolManager f=FishSchoolManager.getInstance();Field seed=FishSchoolManager.class.getDeclaredField("seed");seed.setAccessible(true);
  Field ch=FishSchoolManager.class.getDeclaredField("chumPoints");ch.setAccessible(true);TLongObjectHashMap<FishSchoolManager.ChumData> chum=(TLongObjectHashMap<FishSchoolManager.ChumData>)ch.get(null);
  Field cacheField=FishSchoolManager.class.getDeclaredField("zoneCache");cacheField.setAccessible(true);TLongObjectHashMap<FishSchoolManager.ZoneData> cache=(TLongObjectHashMap<FishSchoolManager.ZoneData>)cacheField.get(null);
  Method hash=FishSchoolManager.class.getDeclaredMethod("coordsToHash",int.class,int.class);hash.setAccessible(true);
  int[][] pos={{0,0},{100,100},{300,300},{600,600},{8306,11692}};
  for(int s:new int[]{1485,12345})for(int[] p:pos)for(int level=1;level<=5;level++)for(int caught:new int[]{0,4})for(int bait=0;bait<=1;bait++){
   seed.setInt(f,s);SandboxOptions.instance.fishAbundance.setValue(level);cache.clear();chum.clear();Zone z=caught==0?null:new Zone("4","Fishing",p[0]-20,p[1]-20,0,40,40);
   for(int dx=-6;dx<=6;dx++)for(int dy=-6;dy<=6;dy++)cache.put((Long)hash.invoke(f,p[0]+dx,p[1]+dy),new FishSchoolManager.ZoneData(z));
   game.setTimeOfDay(10f);if(bait==1)f.addChum(p[0],p[1],200);game.setTimeOfDay(11f);
   System.out.printf("NATURAL %d,%d,%d,%d,%d,%d=%.1f%n",s,p[0],p[1],level,caught,bait,f.getFishAbundance(p[0],p[1]));
  }
 }
}
