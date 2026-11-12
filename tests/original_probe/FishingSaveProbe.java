import java.lang.reflect.Field;
import java.nio.file.*;
import zombie.ZomboidFileSystem;
import zombie.core.Core;
import zombie.iso.FishSchoolManager;
public class FishingSaveProbe {
    public static void main(String[] args) throws Exception {
        Path root = Path.of("/tmp/pz-fishing-save-probe");
        Core.gameMode = "Sandbox"; Core.gameSaveWorld = "oracle";
        ZomboidFileSystem.instance.setCacheDir(root.toString());
        Path dir = Path.of(ZomboidFileSystem.instance.getCurrentSaveDir());
        Files.createDirectories(dir);
        Files.copy(Path.of(args[0]), dir.resolve("fishingData.bin"), StandardCopyOption.REPLACE_EXISTING);
        FishSchoolManager manager = FishSchoolManager.getInstance();
        manager.load();
        Field seed = FishSchoolManager.class.getDeclaredField("seed"); seed.setAccessible(true);
        Field trash = FishSchoolManager.class.getDeclaredField("trashSeed"); trash.setAccessible(true);
        System.out.printf("SAVE seeds=%d,%d%n",seed.getInt(manager),trash.getInt(manager));
        int[][] points = {{8306,11692},{8300,11690},{8200,11700},{8000,11000},{0,0},{2147483600,-2147483600}};
        for (int[] p : points) System.out.printf("SAVE trash=%d,%d,%016x%n",p[0],p[1],Double.doubleToRawLongBits(manager.getTrashAbundance(p[0],p[1])));
        Core.getInstance().setNoSave(false);
        manager.save();
        System.out.printf("SAVE roundtrip=%s%n",Files.mismatch(Path.of(args[0]),dir.resolve("fishingData.bin")) == -1);
    }
}
