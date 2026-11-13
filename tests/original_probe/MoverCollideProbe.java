import java.lang.reflect.Field;
import java.util.ArrayList;
import zombie.core.random.RandStandard;
import zombie.iso.*;
import zombie.network.ServerMap;

// Inside the booted original: the world set-up of SpecialCollideProbe, then objects pushed against walls and doors.
// Args: <scratch dir> <server cell x> <server cell y> then per path: x y vx vy ticks
public class MoverCollideProbe {
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
            if (false) for (IsoGridSquare sq : all) for (int i = 0; i < sq.getObjects().size(); i++) if (sq.getObjects().get(i) instanceof zombie.iso.objects.IsoDoor d) d.ToggleDoorSilent();
            for (IsoGridSquare sq : all) sq.RecalcProperties();
            for (IsoGridSquare sq : all) sq.RecalcAllWithNeighbours(true);
            StringBuilder sb = new StringBuilder();
            // A bare IsoMovingObject (not a character: no polygon collision, no states) pushed with a constant wanted
            // step per tick through the original's own postupdate (DoCollide, feeler, sliding, stairs).
            for (int k = 3; k + 4 < a.length; k += 5) {
                float x = Float.parseFloat(a[k]), y = Float.parseFloat(a[k + 1]);
                float vx = Float.parseFloat(a[k + 2]), vy = Float.parseFloat(a[k + 3]);
                int n = Integer.parseInt(a[k + 4]);
                IsoMovingObject mo = new IsoMovingObject(false);
                mo.setX(x); mo.setY(y); mo.setZ(0.0F);
                mo.setCurrentSquareFromPosition();
                for (int i = 0; i < n; i++) {
                    mo.setLastX(mo.getX());
                    mo.setLastY(mo.getY());
                    mo.setNextX(mo.getX() + vx);
                    mo.setNextY(mo.getY() + vy);
                    mo.postupdate();
                    IsoGridSquare c = mo.getCurrentSquare();
                    sb.append("M ").append(Integer.toHexString(Float.floatToIntBits(mo.getX()))).append(' ').append(Integer.toHexString(Float.floatToIntBits(mo.getY()))).append(' ').append(Integer.toHexString(Float.floatToIntBits(mo.getZ()))).append(' ').append(c == null ? "null" : c.x + "," + c.y + "," + c.z)
                      .append(' ').append(mo.isCollidedN() ? 'N' : '-').append(mo.isCollidedS() ? 'S' : '-').append(mo.isCollidedW() ? 'W' : '-').append(mo.isCollidedE() ? 'E' : '-').append('\n');
                }
            }
            out.print(sb);
        });
    }
}
