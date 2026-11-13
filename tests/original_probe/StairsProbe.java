import java.lang.reflect.Field;
import java.util.ArrayList;
import zombie.core.random.RandStandard;
import zombie.iso.*;
import zombie.network.ServerMap;

// Inside the booted original: the world set-up of SpecialCollideProbe, then stair and slope heights.
// Args: <scratch dir> <server cell x> <server cell y> [toggle]  (toggle: every door is flipped with ToggleDoorSilent first)   (server cell = world square / 64)
public class StairsProbe {
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
            sb.append("F ").append(zombie.core.PerformanceSettings.fboRenderChunk).append('\n');
            // Every square with stairs or a sloped surface: getApparentZ at a 5x5 grid of points (float bits), and the
            // collide-matrix bits towards the level above and below (how one leaves a staircase).
            float[] pts = {0.0F, 0.25F, 0.5F, 0.75F, 1.0F};
            for (IsoGridSquare sq : all) {
                if (!sq.HasStairs() && !sq.hasSlopedSurface()) continue;
                sb.append("Z ").append(sq.x).append(' ').append(sq.y).append(' ').append(sq.z).append(' ').append(sq.getStairs()).append(' ').append(Integer.toHexString(sq.collideMatrix));
                for (float py : pts) for (float px : pts) sb.append(' ').append(Integer.toHexString(Float.floatToIntBits(sq.getApparentZ(px, py))));
                sb.append('\n');
            }
            out.print(sb);
        });
    }
}
