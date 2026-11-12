import java.lang.reflect.Field;
import java.nio.file.*;
import java.util.HexFormat;
import gnu.trove.map.hash.TLongObjectHashMap;
import zombie.ZomboidFileSystem;
import zombie.core.Core;
import zombie.iso.FishSchoolManager;
public class FishingSaveChumProbe {
    @SuppressWarnings("unchecked")
    public static void main(String[] args) throws Exception {
        Core.gameMode="Sandbox"; Core.gameSaveWorld="chum";
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-fishing-chum-probe");
        Files.createDirectories(Path.of(ZomboidFileSystem.instance.getCurrentSaveDir()));
        FishSchoolManager manager=FishSchoolManager.getInstance();
        Field seed=FishSchoolManager.class.getDeclaredField("seed"); seed.setAccessible(true); seed.setInt(manager,12345);
        Field trash=FishSchoolManager.class.getDeclaredField("trashSeed"); trash.setAccessible(true); trash.setInt(manager,54321);
        Field field=FishSchoolManager.class.getDeclaredField("chumPoints"); field.setAccessible(true);
        TLongObjectHashMap<FishSchoolManager.ChumData> map=(TLongObjectHashMap<FishSchoolManager.ChumData>)field.get(null);
        map.clear(); map.put(-123456789012345L,new FishSchoolManager.ChumData(100,200));
        map.put(98765432109876L,new FishSchoolManager.ChumData(-10,987654));
        Core.getInstance().setNoSave(false); manager.save();
        Path file=Path.of(ZomboidFileSystem.instance.getFileNameInCurrentSave("fishingData.bin"));
        System.out.println("CHUM hex="+HexFormat.of().formatHex(Files.readAllBytes(file)));
        seed.setInt(manager,-1); trash.setInt(manager,-1); map.clear(); manager.load();
        System.out.printf("CHUM seeds=%d,%d size=%d%n",seed.getInt(manager),trash.getInt(manager),map.size());
        for(long key:new long[]{-123456789012345L,98765432109876L}) { FishSchoolManager.ChumData value=map.get(key); System.out.printf("CHUM entry=%d,%d,%d%n",key,value.maxForceTime,value.endTime); }
    }
}
