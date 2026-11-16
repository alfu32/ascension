-- unit.railgun_battery@fusion: Railgun battery (mesh.railgun_battery_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.railgun_battery_fusion','Railgun battery');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.railgun_battery_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('track_w','track','mat.graphite',0,'','{"x":-0.62,"l":2.6,"h":0.44,"t":0.32,"wheels":6,"wheel_colour":"#2c2e31"}'),
  ('track_e','track','mat.graphite',0,'','{"x":0.62,"l":2.6,"h":0.44,"t":0.32,"wheels":6,"wheel_colour":"#2c2e31"}'),
  ('hull','box','mat.white_composite',0,'','{"y":0.14,"w":0.94,"d":2.3,"h":0.32}'),
  ('deck','box','mat.white_composite',0,'','{"y":0.46,"w":1.58,"d":2.6,"h":0.05}'),
  ('cab','box','mat.white_composite',0,'','{"y":0.51,"z":0.92,"w":1.3,"d":0.66,"h":0.34,"taper":0.15}'),
  ('windscreen','box','mat.smart_glass',0,'','{"y":0.6,"z":1.22,"w":1,"d":0.03,"h":0.16,"pitch":-20}'),
  ('cab_roof','box','mat.white_composite',1,'','{"y":0.85,"z":0.9,"w":1.06,"d":0.5,"h":0.03}'),
  ('turntable','cylinder','mat.white_composite',0,'','{"y":0.51,"z":-0.42,"w":1.1,"h":0.14,"segments":10,"traverse":20}'),
  ('cradle','box','mat.white_composite',0,'turntable','{"y":0.14,"w":0.58,"d":1,"h":0.3}'),
  ('capacitors','box','mat.white_composite',1,'turntable','{"x":-0.46,"y":0.14,"w":0.26,"d":0.86,"h":0.32,"repeat":2,"dx":0.92}'),
  ('cap_glow','box','mat.plasma',0,'turntable','{"x":-0.6,"y":0.32,"w":0.02,"d":0.66,"h":0.06,"repeat":2,"dx":1.2,"far":0}'),
  ('gun','box','mat.graphite',0,'cradle','{"y":0.22,"z":-0.55,"w":0.2,"d":0.16,"h":3,"pitch":76,"recoil":0.12}'),
  ('rails','box','mat.white_composite',0,'gun','{"x":-0.12,"w":0.05,"d":0.12,"h":3,"repeat":2,"dx":0.24}'),
  ('rail_glow','box','mat.plasma',0,'gun','{"y":0.4,"z":-0.085,"w":0.06,"d":0.02,"h":2.4,"far":0}'),
  ('coil_1','cylinder','mat.plasma',0,'gun','{"y":0.45,"w":0.36,"h":0.06,"segments":6,"far":0}'),
  ('coil_2','cylinder','mat.plasma',0,'gun','{"y":0.95,"w":0.36,"h":0.06,"segments":6,"far":0}'),
  ('muzzle','box','mat.white_composite',0,'gun','{"y":3,"w":0.32,"d":0.22,"h":0.12}'),
  ('spark','cone','mat.plasma',0,'gun','{"y":3.12,"w":0.2,"h":0.2,"segments":3,"flash":1,"far":0}'),
  ('streak','cylinder','mat.light_strip',0,'spark','{"y":0.2,"w":0.06,"h":1.8,"segments":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.5,"w":0.55,"h":0.5,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.railgun_battery_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.railgun_battery'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.railgun_battery_fusion'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
