-- unit.main_battle_tank@information: Main battle tank (mesh.main_battle_tank_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.main_battle_tank_information','Main battle tank');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.main_battle_tank_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('track_w','track','mat.cast_iron',0,'','{"x":-0.64,"l":2.5,"h":0.5,"t":0.36,"wheels":6,"wheel_colour":"#4a503c"}'),
  ('track_e','track','mat.cast_iron',0,'','{"x":0.64,"l":2.5,"h":0.5,"t":0.36,"wheels":6,"wheel_colour":"#4a503c"}'),
  ('hull','box','mat.nato_green',0,'','{"y":0.14,"z":-0.1,"w":0.92,"d":2.1,"h":0.4}'),
  ('glacis','box','mat.nato_green',0,'','{"y":0.16,"z":0.96,"w":0.92,"d":0.05,"h":0.44,"pitch":-72}'),
  ('deck','box','mat.nato_green',0,'','{"y":0.5,"z":-0.05,"w":1.66,"d":2.36,"h":0.06}'),
  ('skirts','box','mat.nato_green',0,'','{"x":-0.83,"y":0.22,"z":0.05,"w":0.04,"d":2.2,"h":0.3,"repeat":2,"dx":1.66}'),
  ('turret','box','mat.nato_green',1,'','{"y":0.56,"z":-0.12,"w":1.12,"d":1,"h":0.28,"taper":0.12,"traverse":22}'),
  ('cheeks','box','mat.nato_green',1,'turret','{"z":0.64,"w":1.06,"d":0.05,"h":0.36,"pitch":-56}'),
  ('bustle','box','mat.nato_green',0,'turret','{"y":0.02,"z":-0.66,"w":1,"d":0.36,"h":0.24,"taper":0.08}'),
  ('sight','box','mat.composite',0,'turret','{"x":0.3,"y":0.26,"z":0.1,"w":0.18,"d":0.2,"h":0.14}'),
  ('sight_lens','box','mat.glass',0,'sight','{"z":0.1,"y":0.03,"w":0.12,"d":0.01,"h":0.08,"tint":"#7fa0c0","far":0}'),
  ('hatch','cylinder','mat.nato_green',0,'turret','{"x":-0.25,"y":0.26,"z":-0.2,"w":0.26,"h":0.05,"segments":7,"far":0}'),
  ('smoke','cylinder','mat.composite',0,'turret','{"x":-0.5,"y":0.16,"z":0.3,"w":0.06,"h":0.12,"segments":4,"pitch":50,"repeat":2,"dx":1,"far":0}'),
  ('mantlet','box','mat.nato_green',0,'turret','{"y":0.04,"z":0.52,"w":0.3,"d":0.12,"h":0.2}'),
  ('gun','cylinder','mat.nato_green',0,'mantlet','{"y":0.1,"w":0.08,"h":1.8,"segments":6,"pitch":90,"recoil":0.14}'),
  ('sleeve','cylinder','mat.nato_green',0,'gun','{"w":0.12,"h":0.85,"segments":6,"far":0}'),
  ('extractor','cylinder','mat.nato_green',0,'gun','{"y":0.95,"w":0.13,"h":0.16,"segments":6,"far":0}'),
  ('spark','cone','mat.flash',0,'gun','{"y":1.8,"w":0.12,"h":0.12,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.6,"w":0.6,"h":0.6,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.main_battle_tank_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.main_battle_tank'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.main_battle_tank_information'), (SELECT id FROM def WHERE key='mat.nato_green'), 100);
