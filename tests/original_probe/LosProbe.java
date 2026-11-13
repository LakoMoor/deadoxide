import java.lang.reflect.Field;
import java.util.ArrayList;
import zombie.core.random.RandStandard;
import zombie.iso.*;
import zombie.network.ServerMap;

// Inside the booted original: the same world set-up as SpecialCollideProbe (one server cell installed in ServerMap,
// optionally all doors flipped), then LosUtil.lineClear from a grid of squares; one digit (TestResults ordinal) per target.
// Args: <scratch dir> <server cell x> <server cell y> [toggle]  (toggle: every door is flipped with ToggleDoorSilent first)   (server cell = world square / 64)
public class LosProbe {
    public static void main(String[] a) throws Exception {
        int scx = Integer.parseInt(a[1]), scy = Integer.parseInt(a[2]);
        ServerHarness.run(a[0], out -> {
            IsoMetaGrid g = new IsoMetaGrid();
            g.Create();
            Field mg = IsoWorld.class.getDeclaredField("metaGrid");
            mg.setAccessible(true);
            mg.set(IsoWorld.instance, g);
            Field rf = zombie.core.random.RandAbstract.class.getDeclaredField("rand");
            rf.setAccessible(true);
            var gp = IsoChunk.class.getDeclaredMethod("getFromPool");
            gp.setAccessible(true);
            IsoCell cell = IsoWorld.instance.currentCell;
            ServerMap.ServerCell sc = new ServerMap.ServerCell();
            sc.wx = scx; sc.wy = scy;
            ArrayList<IsoGridSquare> all = new ArrayList<>();
            for (int cx = 0; cx < 8; cx++) for (int cy = 0; cy < 8; cy++) {
                rf.set(RandStandard.INSTANCE, new java.util.Random(12345L));
                IsoChunk ch = (IsoChunk) gp.invoke(null);
                ch.wx = scx * 8 + cx; ch.wy = scy * 8 + cy;
                CellLoader.LoadCellBinaryChunk(cell, ch.wx, ch.wy, ch);
                sc.chunks[cx][cy] = ch;
                for (int z = ch.getMinLevel(); z <= ch.getMaxLevel(); z++) for (int y = 0; y < 8; y++) for (int x = 0; x < 8; x++) {
                    IsoGridSquare sq = ch.getGridSquare(x, y, z);
                    if (sq != null) all.add(sq);
                }
            }
            sc.isLoaded = true;
            ServerMap m = ServerMap.instance;
            Field wf = ServerMap.class.getDeclaredField("width");
            wf.setAccessible(true);
            m.cellMap[(scy - m.getMinY()) * wf.getInt(m) + (scx - m.getMinX())] = sc;
            if (a.length > 3) for (IsoGridSquare sq : all) for (int i = 0; i < sq.getObjects().size(); i++) if (sq.getObjects().get(i) instanceof zombie.iso.objects.IsoDoor d) d.ToggleDoorSilent();
            for (IsoGridSquare sq : all) sq.RecalcProperties();
            for (IsoGridSquare sq : all) sq.RecalcAllWithNeighbours(true);
            StringBuilder sb = new StringBuilder();
            // From every 8th square on level 0: LosUtil.lineClear to all squares within 10 on levels 0 and 1.
            for (IsoGridSquare sq : all) {
                if (sq.z != 0 || sq.x % 8 != 3 || sq.y % 8 != 3) continue;
                sb.append("L ").append(sq.x).append(' ').append(sq.y).append(' ');
                for (int z = 0; z <= 1; z++) for (int dy = -10; dy <= 10; dy++) for (int dx = -10; dx <= 10; dx++)
                    sb.append(LosUtil.lineClear(cell, sq.x, sq.y, 0, sq.x + dx, sq.y + dy, z, false).ordinal());
                sb.append('\n');
            }
            out.print(sb);
        });
    }
}
