-- bld.ground_battery@starfaring: Ground Battery (mesh.ground_battery_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.ground_battery_starfaring','Ground Battery');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.ground_battery_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','box','mat.steel',0,'','{"w":2.8,"d":2.8,"h":0.45,"taper":0.12,"tint":"#7c838c"}'),
  ('ring','cylinder','mat.iron',0,'','{"y":0.45,"w":1.7,"h":0.3,"segments":12,"tint":"#5d6470"}'),
  ('band','cylinder','mat.steel',1,'','{"y":0.75,"w":1.5,"h":0.12,"segments":12}'),
  ('trim_south','box','mat.steel',1,'','{"y":0.45,"z":1.22,"w":2.5,"d":0.3,"h":0.06}'),
  ('turret','box','mat.steel',0,'','{"y":0.87,"w":1.1,"d":1.2,"h":0.55,"taper":0.2,"tint":"#a6b0bd"}'),
  ('barrel','cylinder','mat.iron',0,'','{"x":-0.22,"y":1.1,"z":0.35,"w":0.16,"h":1.9,"segments":8,"pitch":35,"repeat":2,"dx":0.44,"tint":"#4a5260"}'),
  ('emitter','cylinder','mat.glow',0,'','{"y":1.2,"z":-0.62,"w":0.3,"h":0.06,"segments":8,"pitch":90,"far":0}'),
  ('trim_north','box','mat.steel',1,'','{"y":0.45,"z":-1.22,"w":2.5,"d":0.3,"h":0.06}'),
  ('vent','box','mat.iron',0,'','{"x":-1.05,"y":0.45,"z":-1.05,"w":0.4,"d":0.4,"h":0.2,"repeat":2,"dx":2.1,"tint":"#3d434c","far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.ground_battery_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.ground_battery'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.ground_battery_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
