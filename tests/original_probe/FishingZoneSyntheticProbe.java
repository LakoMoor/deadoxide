import java.nio.*;
import java.util.*;
import zombie.GameWindow;
import zombie.iso.zones.Zone;
import zombie.iso.zones.ZoneGeometryType;
import zombie.util.SharedStrings;
public class FishingZoneSyntheticProbe {
 public static void main(String[] args){
  ByteBuffer b=ByteBuffer.allocate(4096);b.put(new byte[]{'Z','O','N','E'});b.putInt(249);
  String[] strings={"0","Fishing","4","Forest"};b.putInt(strings.length);Map<String,Integer> map=new HashMap<>();
  for(int i=0;i<strings.length;i++){GameWindow.WriteString(b,strings[i]);map.put(strings[i],i);}
  b.putInt(3);
  Zone one=new Zone("4","Fishing",100,200,0,40,40);one.setOriginalName("4");one.id=new UUID(1,2);one.lastActionTimestamp=123456;one.save(b,map);
  Zone two=new Zone("0","Fishing",300,400,0,10,20);two.setOriginalName("0");two.id=new UUID(3,4);two.lastActionTimestamp=654321;two.save(b,map);
  Zone poly=new Zone("0","Forest",500,600,0,10,10);poly.setOriginalName("0");poly.id=new UUID(5,6);poly.geometryType=ZoneGeometryType.Polygon;poly.points.add(500);poly.points.add(600);poly.points.add(510);poly.points.add(600);poly.points.add(510);poly.points.add(610);poly.save(b,map);b.putInt(0);
  byte[] result=Arrays.copyOf(b.array(),b.position());System.out.println("SYNTH hex="+HexFormat.of().formatHex(result));
  b.flip();b.position(8);int n=b.getInt();Map<Integer,String> names=new HashMap<>();for(int i=0;i<n;i++)names.put(i,GameWindow.ReadString(b));int count=b.getInt();SharedStrings shared=new SharedStrings();
  for(int i=0;i<count;i++){Zone z=new Zone().load(b,249,names,shared);if("Fishing".equals(z.type))System.out.printf("SYNTH zone=%s,%d,%d,%d,%d,%d,%s,%d%n",z.name,z.x,z.y,z.z,z.w,z.h,z.geometryType,z.lastActionTimestamp);}
 }
}
