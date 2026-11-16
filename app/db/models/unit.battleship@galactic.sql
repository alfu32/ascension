-- unit.battleship@galactic: Battleship (mesh.battleship_galactic)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.battleship_galactic','Battleship');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.battleship_galactic';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('stern','box','mat.steel',0,'','{"y":0.2,"z":-1.25,"w":2.3,"d":1.6,"h":0.55,"taper":0.12,"tint":"#8e97a3"}'),
  ('hull','box','mat.steel',0,'','{"y":0.2,"z":0.15,"w":1.7,"d":2.2,"h":0.62,"taper":0.12,"tint":"#9aa4b2"}'),
  ('bow','box','mat.steel',0,'','{"y":0.22,"z":1.75,"w":1.15,"d":1.1,"h":0.55,"taper":0.3,"tint":"#a6b0bd"}'),
  ('prow','box','mat.steel',0,'','{"y":0.25,"z":2.5,"w":0.7,"d":0.45,"h":0.45,"taper":0.6,"tint":"#c3cbd6"}'),
  ('flank_plates','box','mat.steel',1,'','{"x":-1.16,"y":0.3,"z":-1.25,"w":0.08,"d":1.5,"h":0.36,"repeat":2,"dx":2.32}'),
  ('flank_stripes','box','mat.steel',1,'','{"x":-0.86,"y":0.55,"z":0.15,"w":0.06,"d":2,"h":0.22,"repeat":2,"dx":1.72,"far":0}'),
  ('deck','box','mat.steel',1,'','{"y":0.82,"z":0,"w":0.4,"d":2.6,"h":0.04,"far":0}'),
  ('tower','box','mat.steel',0,'','{"y":0.82,"z":-0.6,"w":0.8,"d":1,"h":0.45,"taper":0.15,"tint":"#c3cbd6"}'),
  ('tower_upper','box','mat.steel',0,'','{"y":1.27,"z":-0.65,"w":0.55,"d":0.6,"h":0.42,"taper":0.25,"tint":"#c3cbd6"}'),
  ('bridge','box','mat.steel',1,'','{"y":1.69,"z":-0.62,"w":0.7,"d":0.32,"h":0.12}'),
  ('window','box','mat.light_strip',0,'','{"y":1.73,"z":-0.45,"w":0.5,"d":0.02,"h":0.05,"far":0}'),
  ('mast','cylinder','mat.iron',0,'','{"y":1.81,"z":-0.7,"w":0.06,"h":0.6,"segments":4,"tint":"#7c8592","far":0}'),
  ('radar','dome','mat.white_metal',0,'','{"y":2.1,"z":-0.7,"w":0.4,"h":0.08,"segments":8,"pitch":20,"far":0}'),
  ('spinal','cylinder','mat.iron',0,'','{"y":0.8,"z":1.25,"w":0.3,"h":1.7,"segments":8,"pitch":90,"recoil":0.12,"tint":"#4a5260"}'),
  ('spinal_cradle','box','mat.steel',0,'','{"y":0.7,"z":1.6,"w":0.46,"d":0.9,"h":0.16,"tint":"#8e97a3","far":0}'),
  ('spinal_coils','cylinder','mat.plasma',0,'spinal','{"y":0.95,"w":0.38,"h":0.08,"segments":8,"far":0}'),
  ('spinal_coil_2','cylinder','mat.plasma',0,'spinal','{"y":1.3,"w":0.38,"h":0.08,"segments":8,"far":0}'),
  ('spinal_spark','cone','mat.plasma',0,'spinal','{"y":1.72,"w":0.55,"h":0.35,"segments":5,"flash":1,"far":0}'),
  ('spinal_flare','cone','mat.flash',0,'spinal_spark','{"y":0.3,"w":0.7,"h":0.45,"segments":5,"pitch":180,"far":0}'),
  ('spinal_bolt','cylinder','mat.plasma',0,'spinal_spark','{"y":0.3,"w":0.24,"h":1.6,"segments":4,"far":0}'),
  ('engines','cylinder','mat.iron',0,'','{"x":-0.84,"y":0.47,"z":-2.4,"w":0.38,"h":0.4,"segments":6,"pitch":90,"repeat":5,"dx":0.42,"tint":"#4a5260","far":0}'),
  ('drives','wheel','mat.glow',0,'','{"x":-0.84,"y":0.47,"z":-2.42,"yaw":90,"w":0.32,"t":0.05,"spokes":0,"segments":6,"repeat":5,"dx":0.42,"far":0}'),
  ('fore_port','box','mat.steel',0,'','{"x":-0.42,"y":0.82,"z":0.55,"w":0.44,"d":0.42,"h":0.13,"taper":0.25,"traverse":22,"tint":"#9aa4b2"}'),
  ('fore_port_guns','cylinder','mat.iron',0,'fore_port','{"x":-0.11,"y":0.06,"z":0.126,"w":0.055,"h":0.5,"segments":4,"pitch":90,"repeat":3,"dx":0.11,"tint":"#4a5260","far":0}'),
  ('fore_port_flash','box','mat.flash',0,'fore_port','{"y":0.03,"z":0.646,"w":0.38,"d":0.12,"h":0.1,"flash":1,"far":0}'),
  ('fore_port_bolts','cylinder','mat.plasma',0,'fore_port_flash','{"x":-0.11,"y":0.035,"z":0.14,"w":0.07,"h":0.6,"segments":3,"pitch":90,"repeat":3,"dx":0.11,"far":0}'),
  ('fore_starboard','box','mat.steel',0,'','{"x":0.42,"y":0.82,"z":0.55,"w":0.44,"d":0.42,"h":0.13,"taper":0.25,"traverse":22,"tint":"#9aa4b2"}'),
  ('fore_starboard_guns','cylinder','mat.iron',0,'fore_starboard','{"x":-0.11,"y":0.06,"z":0.126,"w":0.055,"h":0.5,"segments":4,"pitch":90,"repeat":3,"dx":0.11,"tint":"#4a5260","far":0}'),
  ('fore_starboard_flash','box','mat.flash',0,'fore_starboard','{"y":0.03,"z":0.646,"w":0.38,"d":0.12,"h":0.1,"flash":1,"far":0}'),
  ('fore_starboard_bolts','cylinder','mat.plasma',0,'fore_starboard_flash','{"x":-0.11,"y":0.035,"z":0.14,"w":0.07,"h":0.6,"segments":3,"pitch":90,"repeat":3,"dx":0.11,"far":0}'),
  ('aft_port','box','mat.steel',0,'','{"x":-0.6,"y":0.75,"z":-1.5,"w":0.36,"d":0.36,"h":0.13,"taper":0.25,"traverse":-25,"tint":"#9aa4b2"}'),
  ('aft_port_guns','cylinder','mat.iron',0,'aft_port','{"x":-0.055,"y":0.06,"z":0.108,"w":0.055,"h":0.4,"segments":4,"pitch":90,"repeat":2,"dx":0.11,"tint":"#4a5260","far":0}'),
  ('aft_port_flash','box','mat.flash',0,'aft_port','{"y":0.03,"z":0.528,"w":0.27,"d":0.12,"h":0.1,"flash":1,"far":0}'),
  ('aft_port_bolts','cylinder','mat.plasma',0,'aft_port_flash','{"x":-0.055,"y":0.035,"z":0.14,"w":0.07,"h":0.6,"segments":3,"pitch":90,"repeat":2,"dx":0.11,"far":0}'),
  ('aft_starboard','box','mat.steel',0,'','{"x":0.6,"y":0.75,"z":-1.5,"w":0.36,"d":0.36,"h":0.13,"taper":0.25,"traverse":25,"tint":"#9aa4b2"}'),
  ('aft_starboard_guns','cylinder','mat.iron',0,'aft_starboard','{"x":-0.055,"y":0.06,"z":0.108,"w":0.055,"h":0.4,"segments":4,"pitch":90,"repeat":2,"dx":0.11,"tint":"#4a5260","far":0}'),
  ('aft_starboard_flash','box','mat.flash',0,'aft_starboard','{"y":0.03,"z":0.528,"w":0.27,"d":0.12,"h":0.1,"flash":1,"far":0}'),
  ('aft_starboard_bolts','cylinder','mat.plasma',0,'aft_starboard_flash','{"x":-0.055,"y":0.035,"z":0.14,"w":0.07,"h":0.6,"segments":3,"pitch":90,"repeat":2,"dx":0.11,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.battleship_galactic'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.battleship'), NULL, (SELECT id FROM def WHERE key='age.galactic'), (SELECT id FROM def WHERE key='mesh.battleship_galactic'), (SELECT id FROM def WHERE key='mat.steel'), 100);
