import java.nio.file.*;
import java.util.*;
import java.lang.reflect.*;
import zombie.ZomboidFileSystem;
import zombie.GameTime;
import zombie.Lua.LuaManager;
import zombie.core.random.RandStandard;
import zombie.iso.FishSchoolManager;
import zombie.util.PZCalendar;
import se.krka.kahlua.luaj.compiler.LuaCompiler;
import gnu.trove.map.hash.TLongObjectHashMap;
public class FishingLureProbe {
 static void run(String s,String n)throws Exception{Object[] r=LuaManager.thread.pcall(LuaCompiler.loadstring(s,n,LuaManager.env),new Object[0]);if(!Boolean.TRUE.equals(r[0]))throw new IllegalStateException(Arrays.toString(r));}
 @SuppressWarnings("unchecked") public static void main(String[] a)throws Exception{
  ZomboidFileSystem.instance.setCacheDir("/tmp/pz-lure-probe");ZomboidFileSystem.instance.init();RandStandard.INSTANCE.init();LuaManager.init();
  run(Files.readString(Path.of(a[0])),"zones");run(Files.readString(Path.of(a[1])),"fish");
  run("Fishing.Utils={skillSizeLimit={[1]=100}}; Fishing.fishes={{name='fish',maxWeight=1,isPredator=false,isLake=true,lure={worm=1}}}; Fishing.trashItems={'trash'}; setGameSpeed=function() end; getGameTime=function() return {setMultiplier=function() end} end; ZombRandFloat=function() return 0.5 end; ZombRand=function() return 0 end; subject={x=8306,y=11692,fishingLvl=1,isRiver=false,isReel=false,lure='worm'}; function selectFish() local item,trash=Fishing.Fish.getFishByLure(subject); outcome=type(item)=='table' and item.name or item; isTrash=trash end","setup");
  GameTime game=new GameTime();GameTime.setInstance(game);GregorianCalendar cal=new GregorianCalendar(TimeZone.getTimeZone("UTC"));cal.clear();game.setCalender(new PZCalendar(cal));game.setYear(1993);game.setMonth(6);game.setDay(8);game.setTimeOfDay(10f);
  FishSchoolManager f=FishSchoolManager.getInstance();Field seed=FishSchoolManager.class.getDeclaredField("seed");seed.setAccessible(true);seed.setInt(f,1485);Field trash=FishSchoolManager.class.getDeclaredField("trashSeed");trash.setAccessible(true);trash.setInt(f,41859);
  Field ch=FishSchoolManager.class.getDeclaredField("chumPoints");ch.setAccessible(true);((TLongObjectHashMap<FishSchoolManager.ChumData>)ch.get(null)).clear();
  run("selectFish()","before");System.out.printf("LURE before=%s,%s%n",LuaManager.env.rawget("outcome"),LuaManager.env.rawget("isTrash"));
  f.addChum(8306,11692,200);game.setTimeOfDay(11f);run("selectFish()","after");System.out.printf("LURE after=%s,%s%n",LuaManager.env.rawget("outcome"),LuaManager.env.rawget("isTrash"));
 }
}
