import java.io.*;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Path;
import zombie.GameTime;
import zombie.core.random.RandStandard;

public class SaveTimeProbe {
    private static void run(String name, byte[] bytes) throws Exception {
        var t = new GameTime();
        try (var in = new DataInputStream(new BufferedInputStream(new ByteArrayInputStream(bytes)))) {
            t.load(in);
            System.out.printf("PROBE %s=%08x,%d,%d,%d,%d,%d,%d%n", name,
                Float.floatToRawIntBits(t.getTimeOfDay()), t.getHour(), t.getMinutes(),
                t.getYear(), t.getMonth(), t.getDay(), in.available());
        }
    }
    public static void main(String[] args) throws Exception {
        RandStandard.INSTANCE.init();
        byte[] original = Files.readAllBytes(Path.of(args[0]));
        run("original", original);
        byte[] current = original.clone(); ByteBuffer.wrap(current).putFloat(24, 19.75f); run("current", current);
        byte[] previous = original.clone(); ByteBuffer.wrap(previous).putFloat(20, 3.5f); run("previous", previous);
        byte[] day = original.clone(); ByteBuffer.wrap(day).putInt(28, 17); run("day", day);
        byte[] month = original.clone(); ByteBuffer.wrap(month).putInt(32, 2); run("month", month);
        byte[] year = original.clone(); ByteBuffer.wrap(year).putInt(36, 1998); run("year", year);
        for (float value : new float[]{0f, 6.1f, 6.999f, 12.001f, 23.999f}) {
            byte[] patch = original.clone(); ByteBuffer.wrap(patch).putFloat(24, value); run("time-" + value, patch);
        }
    }
}
