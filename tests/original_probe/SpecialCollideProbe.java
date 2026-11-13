import java.lang.reflect.Field;
import java.util.ArrayList;
import zombie.core.random.RandStandard;
import zombie.iso.*;
import zombie.network.ServerMap;

// Inside the booted original: loads the 8x8 chunks of one 64-square server cell (like ChunkObjectsProbe) and installs
// them in ServerMap as a loaded ServerCell, so every cell-based lookup (IsoCell.getGridSquare) sees them. Then
// RecalcProperties + RecalcAllWithNeighbours(true) on every square, and prints per square the collide matrix and, for
// the 8 horizontal neighbours, the object returned by testCollideSpecialObjects (a door, window or window frame).
// Args: <scratch dir> <server cell x> <server cell y> [toggle]  (toggle: every door is flipped with ToggleDoorSilent first)   (server cell = world square / 64)
public class SpecialCollideProbe {
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
            int[][] dirs = {{0, -1}, {-1, -1}, {-1, 0}, {-1, 1}, {0, 1}, {1, 1}, {1, 0}, {1, -1}};
            for (IsoGridSquare sq : all) {
                sb.append("P ").append(sq.x).append(' ').append(sq.y).append(' ').append(sq.z).append(' ').append(Integer.toHexString(sq.collideMatrix));
                for (int[] d : dirs) {
                    IsoGridSquare next = cell.getGridSquare(sq.x + d[0], sq.y + d[1], sq.z);
                    IsoObject o = sq.testCollideSpecialObjects(next);
                    sb.append(' ').append(o == null ? "-" : o.getClass().getSimpleName() + ":" + (o.sprite == null ? "null" : o.sprite.name) + "@" + o.square.x + "," + o.square.y);
                }
                sb.append('\n');
            }
            out.print(sb);
        });
    }
}
