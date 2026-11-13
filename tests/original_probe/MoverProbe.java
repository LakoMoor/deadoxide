import java.lang.reflect.Field;
import java.util.ArrayList;
import zombie.core.random.RandStandard;
import zombie.iso.*;
import zombie.network.ServerMap;

// Inside the booted original: the world set-up of SpecialCollideProbe, then a character walked over staircases.
// Args: <scratch dir> <server cell x> <server cell y> then per path: x y dx dy steps (out and back)
public class MoverProbe {
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
            // A real character object (IsoZombie: doStairs rounds the height only for IsoGameCharacter) is moved by
            // hand in steps of 0.1 along the given path; after each step the original's own
            // setCurrentSquareFromPosition, doStairs and snapZToCurrentSquare run. No collision, AI or animation.
            zombie.characters.IsoZombie mo = new zombie.characters.IsoZombie(cell);
            var snap = IsoMovingObject.class.getDeclaredMethod("snapZToCurrentSquare");
            snap.setAccessible(true);
            for (int k = 3; k + 4 < a.length; k += 5) {
                float x = Float.parseFloat(a[k]), y = Float.parseFloat(a[k + 1]);
                float dx = Float.parseFloat(a[k + 2]), dy = Float.parseFloat(a[k + 3]);
                int n = Integer.parseInt(a[k + 4]);
                mo.setX(x); mo.setY(y); mo.setZ(0.0F);
                mo.setCurrentSquareFromPosition();
                for (int leg = 0; leg < 2; leg++) for (int i = 0; i < n; i++) {
                    mo.setLast(mo.getCurrentSquare());
                    mo.setX(mo.getX() + (leg == 0 ? dx : -dx));
                    mo.setY(mo.getY() + (leg == 0 ? dy : -dy));
                    mo.setCurrentSquareFromPosition();
                    mo.doStairs();
                    snap.invoke(mo);
                    IsoGridSquare c = mo.getCurrentSquare();
                    sb.append("M ").append(Integer.toHexString(Float.floatToIntBits(mo.getX()))).append(' ').append(Integer.toHexString(Float.floatToIntBits(mo.getY()))).append(' ').append(Integer.toHexString(Float.floatToIntBits(mo.getZ()))).append(' ').append(c == null ? "null" : c.x + "," + c.y + "," + c.z).append('\n');
                }
            }
            out.print(sb);
        });
    }
}
