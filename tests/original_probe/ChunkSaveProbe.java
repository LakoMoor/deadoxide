import java.io.File;
import java.io.PrintStream;
import java.lang.reflect.Field;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.TreeMap;
import se.krka.kahlua.vm.KahluaTable;
import se.krka.kahlua.vm.KahluaTableIterator;
import zombie.SandboxOptions;
import zombie.core.skinnedmodel.visual.HumanVisual;
import zombie.core.skinnedmodel.visual.ItemVisual;
import zombie.erosion.ErosionData;
import zombie.inventory.InventoryItem;
import zombie.inventory.ItemContainer;
import zombie.iso.IsoChunk;
import zombie.iso.IsoGridSquare;
import zombie.iso.IsoObject;
import zombie.iso.sprite.IsoSprite;
import zombie.iso.sprite.IsoSpriteInstance;

// Inside the ORIGINAL dedicated server (fresh world, seed DeadOxideOracle), the private IsoChunk.LoadFromDiskOrBuffer
// reads every map/<wx>/<wy>.bin of a save; then the chunk, square, object, container and corpse state is printed.
// The save's WorldDictionary.bin must equal the fresh server's (vanilla items, no mods). World-item removal and the
// blood-splat lifespan are switched off so that the result does not depend on the world age. Items print as their
// registry entry (WorldDictionary type) and id (a Moveable takes its world sprite as type), then one I line per item
// with the state its load sets and a J line with the subclass part; a bag's contents follow it. The save's
// recorded_media.bin is loaded first, and the clock is set to a distinctive world age (see below).
//   ChunkSaveProbe <scratch> <save dir>
public class ChunkSaveProbe {
    static Object g(Object o, String name) throws Exception {
        for (Class<?> c = o.getClass(); c != null; c = c.getSuperclass()) {
            try {
                Field f = c.getDeclaredField(name);
                f.setAccessible(true);
                return f.get(o);
            } catch (NoSuchFieldException e) {
                // try the superclass
            }
        }
        throw new NoSuchFieldException(o.getClass().getName() + "." + name);
    }

    static String fh(float f) { return Integer.toHexString(Float.floatToRawIntBits(f)); }
    static String fh(Object f) { return fh(((Float) f).floatValue()); }
    static String dh(double d) { return Long.toHexString(Double.doubleToRawLongBits(d)); }
    static String s(Object o) { return o == null ? "null" : o.toString(); }
    static String b(Object o) { return ((Boolean) o) ? "1" : "0"; }
    static String sn(IsoSprite sp) { return sp == null ? "-" : s(sp.name); }

    static String lua(Object v) {
        if (v instanceof String x) return "s:" + x;
        if (v instanceof Double x) return "n:" + dh(x);
        if (v instanceof Boolean x) return "b:" + x;
        if (v instanceof KahluaTable t) {
            List<String> l = new ArrayList<>();
            KahluaTableIterator it = t.iterator();
            while (it.advance()) l.add(lua(it.getKey()) + "=" + lua(it.getValue()));
            Collections.sort(l);
            return "{" + String.join(",", l) + "}";
        }
        return "?" + v;
    }

    static String color(zombie.core.ImmutableColor c) { return c == null ? "null" : fh(c.r) + "," + fh(c.g) + "," + fh(c.b); }

    static String bytes(byte[] a) {
        if (a == null) return "null";
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < a.length; i++) sb.append(i == 0 ? "" : ",").append(a[i]);
        return sb.toString();
    }

    static void humanVisual(StringBuilder sb, HumanVisual v) throws Exception {
        sb.append("HV\t").append(color((zombie.core.ImmutableColor) g(v, "skinColor"))).append('\t').append(g(v, "skinTexture")).append('\t')
            .append(s(g(v, "skinTextureName"))).append('\t').append(v.zombieRotStage).append('\t').append(color((zombie.core.ImmutableColor) g(v, "hairColor")))
            .append('\t').append(color((zombie.core.ImmutableColor) g(v, "beardColor"))).append('\t').append(color((zombie.core.ImmutableColor) g(v, "naturalHairColor")))
            .append('\t').append(color((zombie.core.ImmutableColor) g(v, "naturalBeardColor"))).append('\t').append(s(g(v, "hairModel"))).append('\t')
            .append(s(g(v, "beardModel"))).append('\t').append(g(v, "bodyHair")).append('\t').append(bytes((byte[]) g(v, "blood"))).append('\t')
            .append(bytes((byte[]) g(v, "dirt"))).append('\t').append(bytes((byte[]) g(v, "holes"))).append('\t').append(s(g(v, "nonAttachedHair"))).append('\n');
        for (ItemVisual iv : (List<ItemVisual>) g(v, "bodyVisuals")) {
            sb.append("IV\t").append(s(g(iv, "fullType"))).append('\t').append(s(g(iv, "alternateModelName"))).append('\t').append(s(g(iv, "clothingItemName")))
                .append('\t').append(color(iv.tint)).append('\t').append(iv.baseTexture).append('\t').append(iv.textureChoice).append('\t').append(fh(iv.hue))
                .append('\t').append(s(iv.decal));
            for (String n : new String[]{"blood", "dirt", "holes", "basicPatches", "denimPatches", "leatherPatches"}) sb.append('\t').append(bytes((byte[]) g(iv, n)));
            sb.append('\n');
        }
    }

    static void container(StringBuilder sb, ItemContainer c) throws Exception {
        sb.append("C\t").append(c.type).append('\t').append(c.explored ? 1 : 0).append('\t').append(b(g(c, "hasBeenLooted"))).append('\t').append(c.capacity);
        for (int i = 0; i < c.getItems().size(); i++) {
            InventoryItem it = c.getItems().get(i);
            sb.append('\t').append(zombie.world.WorldDictionary.getItemTypeFromID(it.getRegistry_id())).append(':').append(it.getID());
        }
        sb.append('\n');
        for (int i = 0; i < c.getItems().size(); i++) item(sb, c.getItems().get(i));
    }

    static String str(Object o) { return o == null ? "null" : "s:" + o; }
    static String flt(Object o) { return fh(o); }

    // One item after InventoryItem.load and its subclass load: the state the save sets or load resets to constants.
    // Not printed: what the item keeps from its script or that synchWithVisual recomputes (see docs/PORT.md E21).
    static void item(StringBuilder sb, InventoryItem it) throws Exception {
        String cls = it.getClass().getSimpleName();
        boolean tinted = it instanceof zombie.inventory.types.Clothing || it instanceof zombie.inventory.types.InventoryContainer;
        boolean moveable = it instanceof zombie.inventory.types.Moveable;
        zombie.core.Color col = (zombie.core.Color) g(it, "col");
        Object name = g(it, "name"), orig = g(it, "originalName");
        java.nio.ByteBuffer bd = (java.nio.ByteBuffer) g(it, "byteData");
        List<?> extra = (List<?>) g(it, "extraItems");
        sb.append("I\t").append(zombie.world.WorldDictionary.getItemTypeFromID(it.getRegistry_id())).append(':').append(it.getID()).append('\t').append(cls)
            .append('\t').append(g(it, "uses")).append('\t').append(g(it, "condition")).append('/').append(g(it, "conditionMax")).append('\t')
            .append(b(g(it, "customColor"))).append('\t').append(tinted ? "~" : fh(col.r) + "," + fh(col.g) + "," + fh(col.b) + "," + fh(col.a)).append('\t')
            .append(flt(g(it, "itemCapacity"))).append('\t').append(b(g(it, "activated"))).append('\t').append(g(it, "haveBeenRepaired")).append('\t')
            .append(moveable || (Short) g(it, "recordedMediaIndex") >= 0 ? "~" : name != null && name.equals(orig) ? "=" : str(name)).append('\t').append(bd == null ? "-" : String.valueOf(bd.limit())).append('\t')
            .append(extra == null ? "-" : String.valueOf(extra)).append('\t').append(b(g(it, "customName"))).append('\t')
            .append(moveable ? "~" : (Boolean) g(it, "customWeight") ? flt(g(it, "actualWeight")) : "-").append('\t').append(g(it, "keyId")).append('\t')
            .append(g(it, "remoteControlId")).append(',').append(g(it, "remoteRange")).append('\t').append(flt(g(it, "colorRed"))).append(',')
            .append(flt(g(it, "colorGreen"))).append(',').append(flt(g(it, "colorBlue"))).append('\t').append(str(g(it, "worker"))).append('\t')
            .append(flt(g(it, "wetCooldown"))).append('\t').append(b(g(it, "favorite"))).append('\t').append(str(g(it, "stashMap"))).append('\t')
            .append(b(g(it, "zombieInfected"))).append('\t').append(g(it, "currentAmmoCount")).append('\t').append(g(it, "attachedSlot")).append('\t')
            .append(str(g(it, "attachedSlotType"))).append('\t').append(str(g(it, "attachedToModel"))).append('\t').append(g(it, "maxCapacity")).append('\t')
            .append(media((Short) g(it, "recordedMediaIndex"))).append('\t').append(flt(g(it, "worldXRotation"))).append(',').append(flt(g(it, "worldYRotation"))).append(',')
            .append(flt(g(it, "worldZRotation"))).append('\t').append(flt(g(it, "worldScale"))).append('\t').append(b(g(it, "isInitialised")));
        KahluaTable t = (KahluaTable) g(it, "table");
        List<String> l = new ArrayList<>();
        Object cn = null;
        if (t != null) {
            KahluaTableIterator ti = t.iterator();
            while (ti.advance()) {
                if ("customName".equals(ti.getKey())) cn = ti.getValue();
                else l.add(lua(ti.getKey()) + "=" + lua(ti.getValue()));
            }
        }
        Collections.sort(l);
        sb.append('\t').append(l.isEmpty() ? "-" : "{" + String.join(",", l) + "}").append('\t').append(cn == null ? "-" : moveable ? "~" : cn.equals(orig) ? "=" : "s:" + cn).append('\n');
        StringBuilder x = new StringBuilder("J");
        switch (cls) {
            case "Key" -> x.append('\t').append(g(it, "numberOfKey"));
            case "AlarmClock", "AlarmClockClothing" -> {
                if (cls.equals("AlarmClockClothing")) clothing(x, it);
                x.append('\t').append(g(it, "alarmHour")).append('\t').append(g(it, "alarmMinutes")).append('\t').append(b(g(it, "alarmSet"))).append('\t')
                    .append(dh((Double) g(it, "ringSince"))).append('\t').append(g(it, "forceDontRing"));
            }
            case "Clothing" -> clothing(x, it);
            case "InventoryContainer" -> x.append('\t').append(g(it, "weightReduction"));
            case "Food" -> {
                Object la = g(it, "lastAged");
                x.append('\t').append(flt(g(it, "age"))).append('\t').append((Float) la == (float) zombie.GameTime.getInstance().getWorldAgeHours() ? "W" : flt(la));
                for (String n : new String[]{"calories", "proteins", "lipids", "carbohydrates", "hungChange", "baseHunger", "unhappyChange", "boredomChange",
                    "thirstChange", "heat", "lastCookMinute", "cookingTime", "cooked", "burnt", "isCookable", "dangerousUncooked", "poisonDetectionLevel", "spices",
                    "poisonPower", "chef", "offAge", "offAgeMax", "painReduction", "fluReduction", "foodSicknessChange", "poison", "useForPoison", "freezingTime",
                    "frozen", "lastFrozenUpdate", "rottenTime", "compostTime", "cookedInMicrowave", "fatigueChange", "endChange", "milkQty", "milkType",
                    "stressChange"}) {
                    Object v = g(it, n);
                    x.append('\t').append(v instanceof Float ? flt(v) : v instanceof Boolean ? b(v) : v instanceof String ? str(v) : String.valueOf(v));
                }
            }
            case "Literature" -> {
                x.append('\t').append(g(it, "numberOfPages")).append('\t').append(g(it, "alreadyReadPages")).append('\t').append(b(g(it, "canBeWrite"))).append('\t')
                    .append(new TreeMap<>((java.util.Map<?, ?>) java.util.Objects.requireNonNullElse(g(it, "customPages"), new java.util.HashMap<>()))).append('\t')
                    .append(str(g(it, "lockedBy"))).append('\t').append(g(it, "learnedRecipes"));
            }
            case "HandWeapon" -> {
                for (String n : new String[]{"maxRange", "minRangeRanged", "clipSize", "minDamage", "maxDamage", "recoilDelay", "aimingTime", "reloadTime",
                    "hitChance", "minAngle", "explosionTimer", "maxAngle", "bloodLevel", "isJammed"}) {
                    Object v = g(it, n);
                    x.append('\t').append(v instanceof Float ? flt(v) : v instanceof Boolean ? b(v) : String.valueOf(v));
                }
                x.append('\t');
                for (Object p : (List<?>) g(it, "attachmentList")) x.append(((InventoryItem) p).getFullType()).append(';');
            }
            case "Moveable", "Radio" -> {
                x.append('\t').append(str(g(it, "worldSprite"))).append('\t').append(b(g(it, "isLight")));
                if ((Boolean) g(it, "isLight")) {
                    x.append('\t').append(b(g(it, "lightUseBattery"))).append('\t').append(b(g(it, "lightHasBattery"))).append('\t').append(str(g(it, "lightBulbItem")));
                    for (String n : new String[]{"lightPower", "lightDelta", "lightR", "lightG", "lightB"}) x.append('\t').append(flt(g(it, n)));
                }
                if (cls.equals("Radio")) {
                    Object d = g(it, "deviceData");
                    x.append('\t').append(s(g(d, "deviceName")));
                    for (String n : new String[]{"twoWay", "transmitRange", "micRange", "micIsMuted", "baseVolumeRange", "deviceVolume", "isPortable", "isTelevision",
                        "isHighTier", "isTurnedOn", "channel", "minChannelRange", "maxChannelRange", "isBatteryPowered", "hasBattery", "powerDelta", "useDelta",
                        "headphoneType", "mediaIndex", "mediaType", "mediaItem", "noTransmit"}) {
                        Object v = g(d, n);
                        x.append('\t').append(v instanceof Float ? fh(v) : v instanceof Boolean ? b(v) : s(v));
                    }
                }
            }
            case "MapItem" -> {
                x.append('\t').append(str(g(it, "mapId")));
                for (Object sy : (List<?>) g(g(it, "symbols"), "symbols")) {
                    x.append('\t').append(sy.getClass().getSimpleName());
                    for (String n : new String[]{"x", "y", "anchorX", "anchorY", "scale", "rotation", "r", "g", "b", "a"}) x.append(',').append(flt(g(sy, n)));
                    x.append(',').append(b(g(sy, "collide"))).append(b(g(sy, "matchPerspective"))).append(b(g(sy, "applyZoom"))).append(',')
                        .append(flt(g(sy, "minZoom"))).append(',').append(flt(g(sy, "maxZoom")));
                    if (sy instanceof zombie.worldMap.symbols.WorldMapTextSymbol) x.append(',').append(str(g(sy, "text"))).append(',').append(b(g(sy, "translated")))
                        .append(',').append(g(sy, "layerId"));
                    else x.append(',').append(str(g(sy, "symbolId")));
                }
            }
            default -> { }
        }
        if (x.length() > 1) sb.append(x).append('\n');
        if (it instanceof zombie.inventory.types.InventoryContainer ic) container(sb, ic.getInventory());
    }

    static String media(short index) {
        if (index < 0) return String.valueOf(index);
        return index + "=" + zombie.radio.ZomboidRadio.getInstance().getRecordedMedia().getMediaDataFromIndex(index).getId();
    }

    // RecordedMedia.load then the Lua registration, as when the game opens the save: the save's ids by index, then the
    // registered ids it lacks (in the server's order).
    @SuppressWarnings("unchecked")
    static void loadRecordedMedia(File save) throws Exception {
        Object rm = zombie.radio.ZomboidRadio.getInstance().getRecordedMedia();
        ArrayList<String> idx = (ArrayList<String>) g(rm, "indexes");
        ByteBuffer bb = ByteBuffer.wrap(Files.readAllBytes(new File(save, "recorded_media.bin").toPath()));
        bb.getInt();
        ArrayList<String> ids = new ArrayList<>();
        for (int i = bb.getInt(); i > 0; i--) ids.add(zombie.GameWindow.ReadString(bb));
        for (String id : idx) if (!ids.contains(id)) ids.add(id);
        idx.clear();
        idx.addAll(ids);
    }

    static void clothing(StringBuilder x, InventoryItem it) throws Exception {
        x.append('\t').append(flt(g(it, "wetness"))).append('\t');
        java.util.Map<?, ?> p = (java.util.Map<?, ?>) g(it, "patches");
        if (p != null) for (var e : new TreeMap<>(p).entrySet()) {
            Object v = e.getValue();
            x.append(e.getKey()).append(':').append(g(v, "tailorLvl")).append('/').append(g(v, "fabricType")).append('/').append(g(v, "scratchDefense")).append('/')
                .append(g(v, "biteDefense")).append('/').append(b(g(v, "hasHole"))).append('/').append(g(v, "conditionGain")).append(';');
        }
    }

    static void object(StringBuilder sb, IsoGridSquare sq, int index, IsoObject o) throws Exception {
        String cls = o.getClass().getSimpleName();
        sb.append("O\t").append(index).append('\t').append(cls).append('\t').append(sn(o.sprite)).append('\t').append(s(g(o, "name"))).append('\t')
            .append(s(g(o, "spriteName"))).append('\t').append(sq.getSpecialObjects().contains(o) ? 1 : 0).append(sq.getWorldObjects().contains(o) ? 1 : 0)
            .append('\t').append(o.isOutlineOnMouseover() ? 1 : 0).append(b(g(o, "doNotSync"))).append(b(g(o, "specialTooltip")))
            .append(b(g(o, "usesExternalWaterSource"))).append(b(g(o, "movedThumpable"))).append('\t').append(fh(o.getRenderYOffset()))
            .append('\t').append(o.getKeyId()).append('\t');
        zombie.core.textures.ColorInfo cc = (zombie.core.textures.ColorInfo) g(o, "customColor");
        sb.append(cc == null ? "-" : fh(cc.r) + "," + fh(cc.g) + "," + fh(cc.b)).append('\t');
        sb.append((Boolean) g(o, "sheetRope") ? fh(g(o, "sheetRopeHealth")) : "-").append('\t');
        IsoSprite ov = (IsoSprite) g(o, "overlaySprite");
        zombie.core.textures.ColorInfo oc = o.getOverlaySpriteColor();
        sb.append(sn(ov)).append('\t').append(oc == null ? "-" : fh(oc.r) + "," + fh(oc.g) + "," + fh(oc.b) + "," + fh(oc.a)).append('\t')
            .append(s(g(o, "spriteModelName"))).append('\t');
        List<Integer> comps = new ArrayList<>();
        for (int i = 0; i < o.componentSize(); i++) comps.add((int) o.getComponentForIndex(i).getComponentType().GetID());
        Collections.sort(comps);
        sb.append(comps).append('\t').append(o.hasModData() ? lua(o.getModData()) : "-").append('\n');
        ArrayList<IsoSpriteInstance> att = (ArrayList<IsoSpriteInstance>) g(o, "attachedAnimSprite");
        if (att != null) for (IsoSpriteInstance a : att) {
            sb.append("A\t").append(sn(a.parentSprite)).append('\t').append(a.flip ? 1 : 0).append(a.copyTargetAlpha ? 1 : 0).append(a.multiplyObjectAlpha ? 1 : 0)
                .append('\t').append(fh(a.offX)).append(',').append(fh(a.offY)).append(',').append(fh(a.offZ)).append('\t').append(fh(a.tintr)).append(',')
                .append(fh(a.tintg)).append(',').append(fh(a.tintb)).append('\t').append(fh(a.alpha)).append('\n');
        }
        ArrayList<?> wbs = (ArrayList<?>) g(o, "wallBloodSplats");
        if (wbs != null) for (Object w : wbs) sb.append("W\t").append(sn((IsoSprite) g(w, "sprite"))).append('\n');
        for (int k = 0; k < o.getContainerCount(); k++) container(sb, o.getContainerByIndex(k));
        StringBuilder x = new StringBuilder("X");
        switch (cls) {
            case "IsoDoor" -> {
                for (String n : new String[]{"open", "locked", "north"}) x.append('\t').append(b(g(o, n)));
                x.append('\t').append(g(o, "health")).append('\t').append(g(o, "maxHealth")).append('\t').append(sn((IsoSprite) g(o, "closedSprite"))).append('\t')
                    .append(sn((IsoSprite) g(o, "openSprite"))).append('\t').append(g(o, "keyId")).append('\t').append(b(g(o, "lockedByKey")));
                for (String n : new String[]{"hasCurtain", "curtainOpen", "curtainInside"}) x.append('\t').append(b(g(o, n)));
            }
            case "IsoWindow" -> {
                x.append('\t').append(b(g(o, "open"))).append('\t').append(b(g(o, "north"))).append('\t').append(g(o, "health"));
                for (String n : new String[]{"locked", "permaLocked", "destroyed", "glassRemoved"}) x.append('\t').append(b(g(o, n)));
                for (String n : new String[]{"openSprite", "closedSprite", "smashedSprite", "glassRemovedSprite"}) x.append('\t').append(sn((IsoSprite) g(o, n)));
                x.append('\t').append(g(o, "maxHealth"));
            }
            case "IsoCurtain" -> x.append('\t').append(b(g(o, "open"))).append('\t').append(b(g(o, "north"))).append('\t').append(g(o, "health")).append('\t')
                .append(g(o, "barricadeStrength")).append('\t').append(sn((IsoSprite) g(o, "closedSprite"))).append('\t').append(sn((IsoSprite) g(o, "openSprite")));
            case "IsoWindowFrame" -> x.append('\t').append(b(g(o, "north")));
            case "IsoTree" -> x.append('\t').append(g(o, "size")).append('\t').append(g(o, "logYield")).append('\t').append(g(o, "damage"));
            case "IsoLightSwitch" -> {
                boolean fromRoom = sq.getRoom() != null && (Boolean) g(o, "lightRoom");
                x.append('\t').append(b(g(o, "lightRoom"))).append('\t').append(g(o, "roomId")).append('\t').append(fromRoom ? "?" : b(g(o, "activated")));
                for (String n : new String[]{"canBeModified", "useBattery", "hasBattery"}) x.append('\t').append(b(g(o, n)));
                x.append('\t').append(s(g(o, "bulbItem")));
                for (String n : new String[]{"power", "delta", "primaryR", "primaryG", "primaryB"}) x.append('\t').append(fh(g(o, n)));
                x.append('\t').append(g(o, "lastMinuteStamp")).append('\t').append(g(o, "bulbBurnMinutes")).append('\t').append(b(g(o, "streetLight")));
            }
            case "IsoBarbecue" -> x.append('\t').append(b(g(o, "hasPropaneTank"))).append('\t').append(g(o, "fuelAmountMinutes")).append('\t').append(b(g(o, "lit")))
                .append('\t').append(fh(g(o, "lastUpdateTime"))).append('\t').append(g(o, "minutesSinceExtinguished")).append('\t')
                .append(sn((IsoSprite) g(o, "normalIsoSprite"))).append('\t').append(sn((IsoSprite) g(o, "noTankIsoSprite")));
            case "IsoFireplace" -> x.append('\t').append(g(o, "fuelAmountMinutes")).append('\t').append(b(g(o, "lit"))).append('\t').append(fh(g(o, "lastUpdateTime")))
                .append('\t').append(g(o, "minutesSinceExtinguished"));
            case "IsoStove" -> x.append('\t').append(b(g(o, "activated"))).append('\t').append(g(o, "secondsTimer")).append('\t').append(fh(g(o, "maxTemperature")))
                .append('\t').append(b(g(o, "firstTurnOn"))).append('\t').append(b(g(o, "broken")));
            case "IsoClothingWasher" -> {
                Object l = g(o, "logic");
                x.append('\t').append(b(g(l, "activated"))).append('\t').append(fh(g(l, "lastUpdate")));
            }
            case "IsoClothingDryer" -> x.append('\t').append(b(g(g(o, "logic"), "activated")));
            case "IsoCombinationWasherDryer" -> {
                Object w = g(o, "washer"), d = g(o, "dryer");
                x.append('\t').append(g(o, "logic") == d ? 1 : 0).append('\t').append(b(g(w, "activated"))).append('\t').append(fh(g(w, "lastUpdate"))).append('\t')
                    .append(b(g(d, "activated")));
            }
            case "IsoRadio", "IsoTelevision" -> {
                Object d = g(o, "deviceData");
                x.append('\t').append(s(g(d, "deviceName")));
                for (String n : new String[]{"twoWay", "transmitRange", "micRange", "micIsMuted", "baseVolumeRange", "deviceVolume", "isPortable", "isTelevision",
                    "isHighTier", "isTurnedOn", "channel", "minChannelRange", "maxChannelRange", "isBatteryPowered", "hasBattery", "powerDelta", "useDelta",
                    "headphoneType", "mediaIndex", "mediaType", "mediaItem", "noTransmit"}) {
                    Object v = g(d, n);
                    x.append('\t').append(v instanceof Float ? fh(v) : v instanceof Boolean ? b(v) : s(v));
                }
                Object p = g(d, "presets");
                x.append('\t').append(g(p, "maxPresets"));
                for (Object e : (List<?>) g(p, "presets")) x.append('\t').append(g(e, "name")).append(':').append(g(e, "frequency"));
            }
            case "IsoMannequin" -> {
                x.append('\t').append(((zombie.iso.objects.IsoMannequin) o).getForwardIsoDirection().ordinal());
                for (String n : new String[]{"init", "female", "zombie", "skeleton"}) x.append('\t').append(b(g(o, n)));
                x.append('\t').append(s(g(o, "mannequinScriptName"))).append('\t').append(s(g(o, "pose")));
            }
            case "IsoWorldInventoryObject" -> {
                zombie.iso.objects.IsoWorldInventoryObject w = (zombie.iso.objects.IsoWorldInventoryObject) o;
                x.append('\t').append(fh(w.xoff)).append('\t').append(fh(w.yoff)).append('\t').append(fh(w.zoff)).append('\t').append(zombie.world.WorldDictionary.getItemTypeFromID(w.getItem().getRegistry_id()))
                    .append(':').append(w.getItem().getID()).append('\t').append(dh(w.dropTime)).append('\t').append(b(g(o, "ignoreRemoveSandbox"))).append('\t')
                    .append(w.isExtendedPlacement() ? 1 : 0);
            }
            default -> { }
        }
        if (x.length() > 1) sb.append(x).append('\n');
        if (cls.equals("IsoMannequin")) humanVisual(sb, (HumanVisual) g(o, "humanVisual"));
        if (cls.equals("IsoWorldInventoryObject")) item(sb, ((zombie.iso.objects.IsoWorldInventoryObject) o).getItem());
    }

    static void corpse(StringBuilder sb, zombie.iso.objects.IsoDeadBody d) throws Exception {
        sb.append("D\t").append(fh(d.getX())).append(',').append(fh(d.getY())).append(',').append(fh(d.getZ())).append('\t').append(d.isFemale() ? 1 : 0)
            .append(b(g(d, "wasZombie"))).append('\t').append(g(g(d, "id"), "id")).append('\t').append(g(d, "persistentOutfitId"));
        for (String n : new String[]{"deathTime", "reanimateTime"}) sb.append('\t').append(fh(g(d, n)));
        for (String n : new String[]{"fallOnFront", "killedByFall", "wasSkeleton"}) sb.append('\t').append(b(g(d, n)));
        sb.append('\t').append(fh(g(d, "angle"))).append('\t').append(g(d, "zombieRotStageAtDeath")).append('\t').append(g(d, "animalRotStageAtDeath")).append('\t')
            .append(s(g(d, "rottenTexture"))).append('\t').append(s(g(d, "skelInvIcon")));
        for (String n : new String[]{"crawling", "fakeDead", "ragdollFall"}) sb.append('\t').append(b(g(d, n)));
        sb.append('\t').append(d.hasModData() ? lua(d.getModData()) : "-").append('\n');
        zombie.characters.SurvivorDesc desc = d.getDescriptor();
        if (desc != null) {
            sb.append("SD\t").append(desc.getID()).append('\t').append(desc.getForename()).append('\t').append(desc.getSurname()).append('\t').append(desc.getTorso())
                .append('\t').append(desc.isFemale() ? 1 : 0).append('\t').append(g(desc, "extra")).append('\t').append(g(desc, "voicePrefix")).append('\t')
                .append(fh(g(desc, "voicePitch"))).append('\t').append(g(desc, "voiceType")).append('\n');
        }
        humanVisual(sb, (HumanVisual) g(d, "baseVisual"));
        if (d.getContainer() != null) container(sb, d.getContainer());
    }

    public static void main(String[] a) throws Exception {
        ServerHarness.run(a[0], out -> {
            SandboxOptions.instance.hoursForWorldItemRemoval.setValue(0.0);
            SandboxOptions.instance.worldItemRemovalList.setValue("");
            SandboxOptions.instance.bloodSplatLifespanDays.setValue(0);
            loadRecordedMedia(new File(a[1]));
            // World age 24000.5 h, so that "W" (Food.load on a server: lastAged -1 becomes the world age) is unambiguous.
            zombie.GameTime.getInstance().setNightsSurvived(1000);
            zombie.GameTime.getInstance().setTimeOfDay(7.5F);
            var gp = IsoChunk.class.getDeclaredMethod("getFromPool");
            gp.setAccessible(true);
            var ld = IsoChunk.class.getDeclaredMethod("LoadFromDiskOrBuffer", ByteBuffer.class);
            ld.setAccessible(true);
            TreeMap<Long, File> files = new TreeMap<>();
            for (File xd : new File(a[1], "map").listFiles()) for (File yf : xd.listFiles()) {
                long wx = Integer.parseInt(xd.getName()), wy = Integer.parseInt(yf.getName().replace(".bin", ""));
                files.put(wx << 32 | wy, yf);
            }
            for (var e : files.entrySet()) {
                int wx = (int) (e.getKey() >> 32), wy = (int) (long) e.getKey();
                StringBuilder sb = new StringBuilder();
                IsoChunk ch = (IsoChunk) gp.invoke(null);
                ch.wx = wx;
                ch.wy = wy;
                try {
                    ld.invoke(ch, ByteBuffer.wrap(Files.readAllBytes(e.getValue().toPath())));
                } catch (java.lang.reflect.InvocationTargetException t) {
                    out.println("K\t" + wx + "\t" + wy + "\tFAIL\t" + t.getCause());
                    continue;
                }
                ErosionData.Chunk ec = ch.getErosionData();
                sb.append("K\t").append(wx).append('\t').append(wy).append('\t').append(ch.getMinLevel()).append('\t').append(ch.getMaxLevel()).append('\t')
                    .append(b(g(ch, "blendingDoneFull"))).append(b(g(ch, "blendingDonePartial")));
                for (boolean f : (boolean[]) g(ch, "blendingModified")) sb.append(f ? 1 : 0);
                sb.append('\t').append(bytes((byte[]) g(ch, "blendingDepth"))).append('\t').append(b(g(ch, "attachmentsDoneFull")));
                for (boolean f : (boolean[]) g(ch, "attachmentsState")) sb.append(f ? 1 : 0);
                List<?> ap = (List<?>) g(ch, "attachmentsPartial");
                sb.append('\t').append(ap == null ? "-" : String.valueOf(ap.size())).append('\t').append(ec.init ? 1 : 0).append(',').append(ec.eTickStamp).append(',')
                    .append(ec.epoch).append(',').append(fh(ec.moisture)).append(',').append(fh(ec.minerals)).append(',').append(ec.soil);
                List<?> gens = (List<?>) g(ch, "generatorsTouchingThisChunk");
                sb.append('\t').append(gens == null ? "-" : String.valueOf(gens.size())).append('\t').append(ch.lootRespawnHour);
                gnu.trove.list.array.TLongArrayList rooms = (gnu.trove.list.array.TLongArrayList) g(ch, "spawnedRooms");
                for (int i = 0; i < rooms.size(); i++) sb.append(i == 0 ? "\t" : ",").append(rooms.get(i));
                sb.append('\n');
                for (int i = 0; i < ch.floorBloodSplats.size(); i++) {
                    var s = ch.floorBloodSplats.get(i);
                    sb.append("B\t").append(fh(s.x)).append(',').append(fh(s.y)).append(',').append(fh(s.z)).append('\t').append(s.type).append('\t').append(s.index)
                        .append('\n');
                }
                for (int z = ch.getMinLevel(); z <= ch.getMaxLevel(); z++) for (int i = 0; i < 64; i++) {
                    IsoGridSquare sq = ch.getGridSquare(i % 8, i / 8, z);
                    if (sq == null) continue;
                    ErosionData.Square es = sq.getErosionData();
                    sb.append("Q\t").append(sq.x).append('\t').append(sq.y).append('\t').append(sq.z).append('\t').append(sq.getRoomID()).append('\t')
                        .append(sq.getRoom() != null ? 1 : 0).append(sq.roofHideBuilding != null ? 1 : 0).append('\t').append(sq.isOverlayDone() ? 1 : 0)
                        .append(sq.haveRoof ? 1 : 0).append(b(g(sq, "burntOut"))).append(sq.haveSheetRope ? 1 : 0).append('\t').append(es.init ? 1 : 0)
                        .append(es.doNothing ? 1 : 0).append(',').append(es.noiseMainByte).append(',').append(es.soil).append(',').append(es.magicNumByte);
                    for (Object r : es.regions) {
                        sb.append(',').append(g(r, "regionId")).append('/').append(g(r, "categoryId")).append('/').append(b(g(r, "doNothing"))).append(b(g(r, "hasSpawned")))
                            .append(b(g(r, "dispBloom"))).append('/').append(g(r, "stage")).append('/').append(g(r, "dispSeason")).append('/').append(g(r, "gameObj"));
                    }
                    sb.append('\t').append(sq.hasModData() ? lua(sq.getModData()) : "-").append('\n');
                    for (int oi = 0; oi < sq.getObjects().size(); oi++) object(sb, sq, oi, sq.getObjects().get(oi));
                    for (int oi = 0; oi < sq.getStaticMovingObjects().size(); oi++) corpse(sb, (zombie.iso.objects.IsoDeadBody) sq.getStaticMovingObjects().get(oi));
                }
                out.print(sb);
            }
        });
    }
}
